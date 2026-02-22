//! Base composition types for schematic records, plus binary pin parsing.
//!
//! All concrete record types derive from one of these two bases:
//!
//! ```text
//! SchPrimitiveBase          (ownership, part/display mode, locking)
//!     |
//!     v
//! SchGraphicalBase          (extends Primitive + location, color, area_color)
//!     |
//!     v
//! Concrete records          (SchRectangle, SchLine, SchArc, etc.)
//! ```
//!
//! Pin records (RECORD=2) are the only binary record type in SchLib. They use a
//! variable-length packed binary format instead of pipe-delimited parameters.
//! `parse_binary_pin` handles them using `BinaryReader`.
//!
//! `SchRecord` is the dispatch enum covering all implemented record variants.
//! `SchLibComponent` bundles a parsed component record with its child records.

use altium_format_derive::FromParams;
use altium_format_types::{
    Color, ComponentKind, Coord, CoordPoint, IeeeSymbol, LineShape, LineStyle,
    ParameterReadOnlyState, ParameterType, PenWidth, PinElectricalType, RotationBy90,
    TextHorzAnchor, TextJustification, TextVertAnchor,
    constants::{
        component::{
            ALL_PIN_COUNT, ALIAS_LIST, COMPONENT_DESCRIPTION, COMPONENT_KIND,
            COMPONENT_KIND_VERSION2, COMPONENT_KIND_VERSION3, CURRENT_PART_ID, DESIGNATOR_LOCKED,
            DISPLAY_FIELD_NAMES, DISPLAY_MODE, DISPLAY_MODE_COUNT, HAS_ONLY_CURRENT_PART_INFO,
            IS_MIRRORED, KEY_COMPONENT_UNIQUE_ID, LIB_REFERENCE, NOT_USE_LIBRARY_NAME,
            PART_COUNT, PART_ID_LOCKED, PINS_MOVEABLE, SHEET_PART_FILE_NAME, SHOW_HIDDEN_FIELDS,
        },
        locking::{GRAPHICALLY_LOCKED, IS_CURRENT, IS_HIDDEN, IS_NOT_ACCESSIBLE, NOT_AUTO_POSITION, READ_ONLY_STATE},
        model::{
            DATABASE_DATALINKS_LOCKED, DATABASE_MODEL, DATAFILE_COUNT, DATALINKS_LOCKED,
            DES_IMP_COUNT, DES_INTF, INTEGRATED_MODEL, MODEL_ITEM_GUID, MODEL_LOCATION, MODEL_NAME,
            MODEL_REVISION_GUID, MODEL_TYPE, MODEL_VAULT_GUID, USE_COMPONENT_LIBRARY,
        },
        parsing::C_BASE_UNIT,
        pin::{
            PIN_COLOR, PIN_CONGLOMERATE_GRAPHICALLY_LOCKED, PIN_CONGLOMERATE_IS_HIDDEN,
            PIN_CONGLOMERATE_NOT_ACCESSIBLE, PIN_CONGLOMERATE_ORIENTATION_MASK,
            PIN_CONGLOMERATE_OWNER_INDEX_ADDITIONAL_LIST, PIN_CONGLOMERATE_SHOW_DESIGNATOR,
            PIN_CONGLOMERATE_SHOW_NAME,
        },
        record_structure::{
            INDEX_IN_SHEET, IS_IMAGE_PARAMETER, OWNER_INDEX, OWNER_PART_DISPLAY_MODE, OWNER_PART_ID,
            PARAM_TYPE, UNIQUE_ID, URL,
        },
        sheet::{AREA_COLOR, SHOW_BORDER, SHOW_HIDDEN_PINS, TARGET_FILE_NAME},
        text::{
            ALIGNMENT, CLIP_TO_RECT, DESCRIPTION, JUSTIFICATION, NAME, SHOW_NAME, TEXT,
            TEXT_COLOR, TEXT_HORZ_ANCHOR, TEXT_MARGIN, TEXT_MARGIN_FRAC, TEXT_VERT_ANCHOR, WORD_WRAP,
        },
        vault::{
            DATABASE_TABLE_NAME, DESIGN_ITEM_ID, GENERIC_COMPONENT_TEMPLATE_GUID, ITEM_GUID,
            LIBRARY_PATH, NOT_ALLOW_DATABASE_SYNCHRONIZE, NOT_ALLOW_LIBRARY_SYNCHRONIZE,
            NOT_USE_DB_TABLE_NAME, REVISION_GUID, SOURCE_LIBRARY_NAME,
            SYMBOL_ITEM_GUID, SYMBOL_REVISION_GUID, SYMBOL_VAULT_GUID, VAULT_GUID,
        },
        visual::{
            COLOR, CORNER_X, CORNER_X_FRAC, CORNER_X_RADIUS, CORNER_X_RADIUS_FRAC, CORNER_Y,
            CORNER_Y_FRAC, CORNER_Y_RADIUS, CORNER_Y_RADIUS_FRAC, EMBED_IMAGE, END_ANGLE,
            END_LINE_SHAPE, FILE_NAME, FONT_ID, IS_SOLID, KEEP_ASPECT, LINE_SHAPE_SIZE,
            LINE_STYLE, LINE_STYLE_EXT, LINE_WIDTH, LOCATION_COUNT,
            LOCATION_X, LOCATION_X_FRAC, LOCATION_Y, LOCATION_Y_FRAC, ORIENTATION, OVERIDE_COLORS,
            RADIUS, RADIUS_FRAC, SECONDARY_RADIUS, SECONDARY_RADIUS_FRAC, START_ANGLE,
            START_LINE_SHAPE, TRANSPARENT,
        },
    },
};

use crate::binary_io::BinaryReader;
use crate::param_collection::ParameterCollection;
use crate::{AltiumFormatError, Result};

// ── Base composition types ────────────────────────────────────────────────────

/// Base fields shared by every schematic record: ownership, part/display mode, locking.
#[derive(FromParams, Debug)]
pub(crate) struct SchPrimitiveBase {
    #[param(key = OWNER_INDEX, default = -1i32)]
    pub owner_index: i32,
    #[param(key = IS_NOT_ACCESSIBLE, default = false)]
    pub is_not_accessible: bool,
    #[param(key = OWNER_PART_ID, default = -1i32)]
    pub owner_part_id: i32,
    #[param(key = OWNER_PART_DISPLAY_MODE, default = 0i32)]
    pub owner_part_display_mode: i32,
    #[param(key = GRAPHICALLY_LOCKED, default = false)]
    pub graphically_locked: bool,
    #[param(key = INDEX_IN_SHEET, default = -1i32)]
    pub index_in_sheet: i32,
}

/// Extends `SchPrimitiveBase` with location and color fields for graphical objects.
#[derive(FromParams, Debug)]
pub(crate) struct SchGraphicalBase {
    #[param(flatten)]
    pub primitive: SchPrimitiveBase,
    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = AREA_COLOR, default = Color::BLACK)]
    pub area_color: Color,
}

// ── Binary pin types ──────────────────────────────────────────────────────────

/// Text positioning override data for a pin's name or designator label.
///
/// Populated from the `PinTextData` sidecar stream (Milestone 9).
#[derive(Debug, Clone)]
pub(crate) struct PinTextPositioning {
    pub position_mode_custom: bool,
    pub rotation_anchor_component: bool,
    pub rotation_relative: RotationBy90,
    pub font_mode_custom: bool,
    pub custom_position_margin: Option<Coord>,
    pub custom_font_id: Option<i16>,
    pub custom_color: Option<Color>,
}

/// Fields extracted from the `PinConglomerate` byte.
struct PinConglomerateFields {
    orientation: RotationBy90,
    is_hidden: bool,
    show_name: bool,
    show_designator: bool,
    is_not_accessible: bool,
    graphically_locked: bool,
    owner_index_additional_list: bool,
}

/// Decodes the `PinConglomerate` bitmask byte into individual fields.
fn decode_pin_conglomerate(byte: u8) -> Result<PinConglomerateFields> {
    let orientation_raw = byte & PIN_CONGLOMERATE_ORIENTATION_MASK;
    let orientation = RotationBy90::try_from(orientation_raw)?;
    Ok(PinConglomerateFields {
        orientation,
        is_hidden: (byte & PIN_CONGLOMERATE_IS_HIDDEN) != 0,
        show_name: (byte & PIN_CONGLOMERATE_SHOW_NAME) != 0,
        show_designator: (byte & PIN_CONGLOMERATE_SHOW_DESIGNATOR) != 0,
        is_not_accessible: (byte & PIN_CONGLOMERATE_NOT_ACCESSIBLE) != 0,
        graphically_locked: (byte & PIN_CONGLOMERATE_GRAPHICALLY_LOCKED) != 0,
        owner_index_additional_list: (byte & PIN_CONGLOMERATE_OWNER_INDEX_ADDITIONAL_LIST) != 0,
    })
}

/// A parsed schematic pin (RECORD=2).
///
/// Pins use a variable-length binary format with length-prefixed ASCII string
/// fields. Sidecar fields are zero-initialized here and populated in Milestone 9.
#[derive(Debug)]
pub(crate) struct SchPin {
    // Decoded from binary
    pub owner_index: i32,
    pub owner_part_id: i32,
    pub owner_part_display_mode: u8,
    pub symbol_inner_edge: IeeeSymbol,
    pub symbol_outer_edge: IeeeSymbol,
    pub symbol_inside: IeeeSymbol,
    pub symbol_outside: IeeeSymbol,
    pub description: String,
    pub formal_type: u8,
    pub electrical: PinElectricalType,
    pub pin_length: Coord,
    pub location: CoordPoint,
    pub color: Color,
    pub name: String,
    pub designator: String,
    pub swap_id_pin: String,
    pub swap_id_part: String,
    pub default_value: String,

    // Decoded from PinConglomerate
    pub orientation: RotationBy90,
    pub is_hidden: bool,
    pub show_name: bool,
    pub show_designator: bool,
    pub is_not_accessible: bool,
    pub graphically_locked: bool,
    pub owner_index_additional_list: bool,

    // Populated by sidecar streams (Milestone 9)
    pub pin_symbol_line_width: i32,
    pub pin_package_length: String,
    pub propagation_delay: String,
    pub selected_functions: Vec<String>,
    pub defined_functions: Vec<String>,

    // Text positioning (from PinTextData sidecar, Milestone 9)
    pub name_text_data: Option<PinTextPositioning>,
    pub designator_text_data: Option<PinTextPositioning>,
}

/// Parses one binary pin record from raw block payload bytes.
///
/// Layout (per `FileFormatV5.cs` `ImportPin`):
/// ```text
/// 0x00       u8      binary_code             Must be 0x02
/// 0x01       i32LE   owner_index
/// 0x05       i16LE   owner_part_id
/// 0x07       u8      owner_part_display_mode
/// 0x08       u8      symbol_inner_edge       IeeeSymbol
/// 0x09       u8      symbol_outer_edge       IeeeSymbol
/// 0x0A       u8      symbol_inside           IeeeSymbol
/// 0x0B       u8      symbol_outside          IeeeSymbol
/// 0x0C       u8      description_length      N
/// 0x0D       N       description             ASCII
/// 0x0D+N     u8      formal_type
/// 0x0E+N     u8      electrical              PinElectricalType
/// 0x0F+N     u8      pin_conglomerate
/// 0x10+N     i16LE   pin_length              DXP units (× C_BASE_UNIT)
/// 0x12+N     i16LE   location_x              DXP units
/// 0x14+N     i16LE   location_y              DXP units
/// 0x16+N     i32LE   color                   Win32 COLORREF
/// 0x1A+N     u8      name_length             M
/// 0x1B+N     M       name                    ASCII
/// 0x1B+N+M   u8      designator_length       P
/// 0x1C+N+M   P       designator              ASCII
/// 0x1C+N+M+P u8      swap_id_pin_length      Q
/// +Q         u8      swap_id_part_length     R
/// +R         u8      default_value_length    S
/// +S                 (end)
/// ```
pub(crate) fn parse_binary_pin(data: &[u8]) -> Result<SchPin> {
    let mut r = BinaryReader::new(data);

    let binary_code = r.read_u8()?;
    if binary_code != 0x02 {
        return Err(AltiumFormatError::UnknownBinaryCode(binary_code));
    }

    let owner_index = r.read_i32_le()?;
    let owner_part_id = r.read_i16_le()? as i32;
    let owner_part_display_mode = r.read_u8()?;

    let symbol_inner_edge = IeeeSymbol::try_from(r.read_u8()?)?;
    let symbol_outer_edge = IeeeSymbol::try_from(r.read_u8()?)?;
    let symbol_inside = IeeeSymbol::try_from(r.read_u8()?)?;
    let symbol_outside = IeeeSymbol::try_from(r.read_u8()?)?;

    let description = r.read_pascal_string()?;

    let formal_type = r.read_u8()?;
    let electrical = PinElectricalType::try_from(r.read_u8()?)?;
    let conglomerate_byte = r.read_u8()?;
    let cong = decode_pin_conglomerate(conglomerate_byte)?;

    let pin_length_raw = r.read_i16_le()?;
    let pin_length = Coord::from_internal(pin_length_raw as i32 * C_BASE_UNIT);

    let location_x_raw = r.read_i16_le()?;
    let location_y_raw = r.read_i16_le()?;
    let location = CoordPoint::new(
        Coord::from_internal(location_x_raw as i32 * C_BASE_UNIT),
        Coord::from_internal(location_y_raw as i32 * C_BASE_UNIT),
    );

    let color_raw = r.read_i32_le()?;
    let color = Color::new(color_raw);

    let name = r.read_pascal_string()?;
    let designator = r.read_pascal_string()?;
    let swap_id_pin = r.read_pascal_string()?;
    let swap_id_part = r.read_pascal_string()?;
    let default_value = r.read_pascal_string()?;

    r.assert_exhausted()?;

    Ok(SchPin {
        owner_index,
        owner_part_id,
        owner_part_display_mode,
        symbol_inner_edge,
        symbol_outer_edge,
        symbol_inside,
        symbol_outside,
        description,
        formal_type,
        electrical,
        pin_length,
        location,
        color,
        name,
        designator,
        swap_id_pin,
        swap_id_part,
        default_value,
        orientation: cong.orientation,
        is_hidden: cong.is_hidden,
        show_name: cong.show_name,
        show_designator: cong.show_designator,
        is_not_accessible: cong.is_not_accessible,
        graphically_locked: cong.graphically_locked,
        owner_index_additional_list: cong.owner_index_additional_list,
        // Sidecar fields: zero-initialized, populated in M9
        pin_symbol_line_width: 0,
        pin_package_length: String::new(),
        propagation_delay: String::new(),
        selected_functions: Vec::new(),
        defined_functions: Vec::new(),
        name_text_data: None,
        designator_text_data: None,
    })
}

// ── SchComponent (RECORD=1) ───────────────────────────────────────────────────

/// A schematic component record (RECORD=1).
///
/// Parsed from the first block of each component's `/<key>/Data` stream.
/// Fields follow the invariant order defined in `FileFormatV5.cs`.
#[derive(Debug)]
pub(crate) struct SchComponent {
    pub lib_reference: String,
    pub component_description: String,
    pub part_count: i32,
    pub display_mode_count: i32,
    pub owner_index: i32,
    pub is_not_accessible: bool,
    pub owner_part_id: i32,
    pub owner_part_display_mode: i32,
    pub graphically_locked: bool,
    pub index_in_sheet: i32,
    pub location: CoordPoint,
    pub display_mode: i32,
    pub is_mirrored: bool,
    pub orientation: RotationBy90,
    pub current_part_id: i32,
    pub show_hidden_fields: bool,
    pub show_hidden_pins: bool,
    pub library_path: String,
    pub source_library_name: String,
    pub database_table_name: String,
    pub sheet_part_file_name: String,
    pub target_file_name: String,
    pub unique_id: String,
    pub area_color: Color,
    pub color: Color,
    pub pin_color: Color,
    pub override_colors: bool,
    pub display_field_names: bool,
    pub designator_locked: bool,
    pub part_id_locked: bool,
    pub pins_moveable: bool,
    pub alias_list: String,
    pub not_use_library_name: bool,
    pub not_use_db_table_name: bool,
    pub design_item_id: String,
    pub vault_guid: String,
    pub item_guid: String,
    pub revision_guid: String,
    pub symbol_vault_guid: String,
    pub symbol_item_guid: String,
    pub symbol_revision_guid: String,
    pub generic_component_template_guid: String,
    pub has_only_current_part_info: bool,
    pub all_pin_count: i32,
    pub key_component_unique_id: String,
    pub component_kind: ComponentKind,
    pub component_kind_version2: ComponentKind,
    pub component_kind_version3: ComponentKind,
    pub custom_display_mode_names: Vec<String>,
}

pub(crate) fn parse_component_record(params: &mut crate::param_collection::ParameterCollection) -> crate::Result<SchComponent> {
    let lib_reference: String = params.remove_with_default(LIB_REFERENCE, String::new())?;
    let component_description: String = params.remove_with_default(COMPONENT_DESCRIPTION, String::new())?;
    let part_count: i32 = params.remove_with_default(PART_COUNT, 1i32)?;
    let display_mode_count: i32 = params.remove_with_default(DISPLAY_MODE_COUNT, 0i32)?;

    // ExportGraphicalObject fields
    let owner_index: i32 = params.remove_with_default(OWNER_INDEX, -1i32)?;
    let is_not_accessible: bool = params.remove_with_default(IS_NOT_ACCESSIBLE, false)?;
    let owner_part_id: i32 = params.remove_with_default(OWNER_PART_ID, -1i32)?;
    let owner_part_display_mode: i32 = params.remove_with_default(OWNER_PART_DISPLAY_MODE, 0i32)?;
    let graphically_locked: bool = params.remove_with_default(GRAPHICALLY_LOCKED, false)?;
    let index_in_sheet: i32 = params.remove_with_default(INDEX_IN_SHEET, -1i32)?;

    // Location (DXP frac coords)
    let location_x: i32 = params.remove_with_default(LOCATION_X, 0i32)?;
    let location_x_frac: i32 = params.remove_with_default(LOCATION_X_FRAC, 0i32)?;
    let location_y: i32 = params.remove_with_default(LOCATION_Y, 0i32)?;
    let location_y_frac: i32 = params.remove_with_default(LOCATION_Y_FRAC, 0i32)?;
    let location = CoordPoint::new(
        Coord::from_dxp_frac(location_x, location_x_frac),
        Coord::from_dxp_frac(location_y, location_y_frac),
    );

    let display_mode: i32 = params.remove_with_default(DISPLAY_MODE, 0i32)?;
    let is_mirrored: bool = params.remove_with_default(IS_MIRRORED, false)?;
    let orientation: RotationBy90 = params.remove_with_default(ORIENTATION, RotationBy90::Rotate0)?;
    let current_part_id: i32 = params.remove_with_default(CURRENT_PART_ID, 1i32)?;
    let show_hidden_fields: bool = params.remove_with_default(SHOW_HIDDEN_FIELDS, false)?;
    let show_hidden_pins: bool = params.remove_with_default(SHOW_HIDDEN_PINS, false)?;
    let library_path: String = params.remove_with_default(LIBRARY_PATH, String::new())?;
    let source_library_name: String = params.remove_with_default(SOURCE_LIBRARY_NAME, String::new())?;
    let database_table_name: String = params.remove_with_default(DATABASE_TABLE_NAME, String::new())?;
    let sheet_part_file_name: String = params.remove_with_default(SHEET_PART_FILE_NAME, String::new())?;
    let target_file_name: String = params.remove_with_default(TARGET_FILE_NAME, String::new())?;
    let unique_id: String = params.remove_with_default(UNIQUE_ID, String::new())?;
    let area_color: Color = params.remove_with_default(AREA_COLOR, Color::BLACK)?;
    let color: Color = params.remove_with_default(COLOR, Color::BLACK)?;
    let pin_color: Color = params.remove_with_default(PIN_COLOR, Color::BLACK)?;
    let override_colors: bool = params.remove_with_default(OVERIDE_COLORS, false)?;
    let display_field_names: bool = params.remove_with_default(DISPLAY_FIELD_NAMES, false)?;
    let designator_locked: bool = params.remove_with_default(DESIGNATOR_LOCKED, false)?;
    let part_id_locked: bool = params.remove_with_default(PART_ID_LOCKED, false)?;
    let pins_moveable: bool = params.remove_with_default(PINS_MOVEABLE, false)?;
    let alias_list: String = params.remove_with_default(ALIAS_LIST, String::new())?;
    let not_use_library_name: bool = params.remove_with_default(NOT_USE_LIBRARY_NAME, false)?;
    let not_use_db_table_name: bool = params.remove_with_default(NOT_USE_DB_TABLE_NAME, false)?;
    let design_item_id: String = params.remove_with_default(DESIGN_ITEM_ID, String::new())?;
    let vault_guid: String = params.remove_with_default(VAULT_GUID, String::new())?;
    let item_guid: String = params.remove_with_default(ITEM_GUID, String::new())?;
    let revision_guid: String = params.remove_with_default(REVISION_GUID, String::new())?;
    let symbol_vault_guid: String = params.remove_with_default(SYMBOL_VAULT_GUID, String::new())?;
    let symbol_item_guid: String = params.remove_with_default(SYMBOL_ITEM_GUID, String::new())?;
    let symbol_revision_guid: String = params.remove_with_default(SYMBOL_REVISION_GUID, String::new())?;
    let generic_component_template_guid: String = params.remove_with_default(GENERIC_COMPONENT_TEMPLATE_GUID, String::new())?;
    let has_only_current_part_info: bool = params.remove_with_default(HAS_ONLY_CURRENT_PART_INFO, false)?;
    let all_pin_count: i32 = params.remove_with_default(ALL_PIN_COUNT, 0i32)?;
    let key_component_unique_id: String = params.remove_with_default(KEY_COMPONENT_UNIQUE_ID, String::new())?;
    let component_kind: ComponentKind = params.remove_with_default(COMPONENT_KIND, ComponentKind::Standard)?;
    let component_kind_version2: ComponentKind = params.remove_with_default(COMPONENT_KIND_VERSION2, ComponentKind::Standard)?;
    let component_kind_version3: ComponentKind = params.remove_with_default(COMPONENT_KIND_VERSION3, ComponentKind::Standard)?;

    // CustomDisplayModeName0..N-1
    let mut custom_display_mode_names = Vec::with_capacity(display_mode_count as usize);
    for i in 0..display_mode_count {
        let key = format!("{}{}", altium_format_types::constants::component::CUSTOM_DISPLAY_MODE_NAME, i);
        let name: String = params.remove_with_default(&key, String::new())?;
        custom_display_mode_names.push(name);
    }

    Ok(SchComponent {
        lib_reference,
        component_description,
        part_count,
        display_mode_count,
        owner_index,
        is_not_accessible,
        owner_part_id,
        owner_part_display_mode,
        graphically_locked,
        index_in_sheet,
        location,
        display_mode,
        is_mirrored,
        orientation,
        current_part_id,
        show_hidden_fields,
        show_hidden_pins,
        library_path,
        source_library_name,
        database_table_name,
        sheet_part_file_name,
        target_file_name,
        unique_id,
        area_color,
        color,
        pin_color,
        override_colors,
        display_field_names,
        designator_locked,
        part_id_locked,
        pins_moveable,
        alias_list,
        not_use_library_name,
        not_use_db_table_name,
        design_item_id,
        vault_guid,
        item_guid,
        revision_guid,
        symbol_vault_guid,
        symbol_item_guid,
        symbol_revision_guid,
        generic_component_template_guid,
        has_only_current_part_info,
        all_pin_count,
        key_component_unique_id,
        component_kind,
        component_kind_version2,
        component_kind_version3,
        custom_display_mode_names,
    })
}

// ── Graphical primitive records ───────────────────────────────────────────────

/// A line segment (RECORD=13).
#[derive(FromParams, Debug)]
pub(crate) struct SchLine {
    #[param(flatten)]
    pub base: SchGraphicalBase,
    #[param(coord_point, x_key = CORNER_X, x_frac = CORNER_X_FRAC, y_key = CORNER_Y, y_frac = CORNER_Y_FRAC)]
    pub corner: CoordPoint,
    #[param(key = LINE_WIDTH, default = PenWidth::Zero)]
    pub line_width: PenWidth,
    #[param(key = LINE_STYLE, default = LineStyle::Solid)]
    pub line_style: LineStyle,
    #[param(key = LINE_STYLE_EXT, default = LineStyle::Solid)]
    pub line_style_ext: LineStyle,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}

/// A filled or outlined rectangle (RECORD=14).
#[derive(FromParams, Debug)]
pub(crate) struct SchRectangle {
    #[param(flatten)]
    pub base: SchGraphicalBase,
    #[param(coord_point, x_key = CORNER_X, x_frac = CORNER_X_FRAC, y_key = CORNER_Y, y_frac = CORNER_Y_FRAC)]
    pub corner: CoordPoint,
    #[param(key = LINE_WIDTH, default = PenWidth::Zero)]
    pub line_width: PenWidth,
    #[param(key = LINE_STYLE_EXT, default = LineStyle::Solid)]
    pub line_style: LineStyle,
    #[param(key = IS_SOLID, default = false)]
    pub is_solid: bool,
    #[param(key = TRANSPARENT, default = false)]
    pub transparent: bool,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}

/// A rounded rectangle (RECORD=10).
#[derive(FromParams, Debug)]
pub(crate) struct SchRoundRectangle {
    #[param(flatten)]
    pub base: SchGraphicalBase,
    #[param(coord_point, x_key = CORNER_X, x_frac = CORNER_X_FRAC, y_key = CORNER_Y, y_frac = CORNER_Y_FRAC)]
    pub corner: CoordPoint,
    #[param(key = LINE_WIDTH, default = PenWidth::Zero)]
    pub line_width: PenWidth,
    #[param(key = IS_SOLID, default = false)]
    pub is_solid: bool,
    #[param(coord, key = CORNER_X_RADIUS, frac_key = CORNER_X_RADIUS_FRAC)]
    pub corner_x_radius: Coord,
    #[param(coord, key = CORNER_Y_RADIUS, frac_key = CORNER_Y_RADIUS_FRAC)]
    pub corner_y_radius: Coord,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}

/// A circular arc segment (RECORD=12).
#[derive(FromParams, Debug)]
pub(crate) struct SchArc {
    #[param(flatten)]
    pub base: SchGraphicalBase,
    #[param(coord, key = RADIUS, frac_key = RADIUS_FRAC)]
    pub radius: Coord,
    #[param(key = START_ANGLE, default = 0.0f64)]
    pub start_angle: f64,
    #[param(key = END_ANGLE, default = 360.0f64)]
    pub end_angle: f64,
    #[param(key = LINE_WIDTH, default = PenWidth::Zero)]
    pub line_width: PenWidth,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}

/// An elliptical arc segment (RECORD=11).
#[derive(FromParams, Debug)]
pub(crate) struct SchEllipticalArc {
    #[param(flatten)]
    pub base: SchGraphicalBase,
    #[param(coord, key = RADIUS, frac_key = RADIUS_FRAC)]
    pub radius: Coord,
    #[param(coord, key = SECONDARY_RADIUS, frac_key = SECONDARY_RADIUS_FRAC)]
    pub secondary_radius: Coord,
    #[param(key = START_ANGLE, default = 0.0f64)]
    pub start_angle: f64,
    #[param(key = END_ANGLE, default = 360.0f64)]
    pub end_angle: f64,
    #[param(key = LINE_WIDTH, default = PenWidth::Zero)]
    pub line_width: PenWidth,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}

/// A filled or outlined ellipse (RECORD=8).
#[derive(FromParams, Debug)]
pub(crate) struct SchEllipse {
    #[param(flatten)]
    pub base: SchGraphicalBase,
    #[param(coord, key = RADIUS, frac_key = RADIUS_FRAC)]
    pub radius: Coord,
    #[param(coord, key = SECONDARY_RADIUS, frac_key = SECONDARY_RADIUS_FRAC)]
    pub secondary_radius: Coord,
    #[param(key = LINE_WIDTH, default = PenWidth::Zero)]
    pub line_width: PenWidth,
    #[param(key = IS_SOLID, default = false)]
    pub is_solid: bool,
    #[param(key = TRANSPARENT, default = false)]
    pub transparent: bool,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}

/// A pie (filled wedge) shape (RECORD=9).
#[derive(FromParams, Debug)]
pub(crate) struct SchPie {
    #[param(flatten)]
    pub base: SchGraphicalBase,
    #[param(coord, key = RADIUS, frac_key = RADIUS_FRAC)]
    pub radius: Coord,
    #[param(coord, key = SECONDARY_RADIUS, frac_key = SECONDARY_RADIUS_FRAC)]
    pub secondary_radius: Coord,
    #[param(key = START_ANGLE, default = 0.0f64)]
    pub start_angle: f64,
    #[param(key = END_ANGLE, default = 360.0f64)]
    pub end_angle: f64,
    #[param(key = LINE_WIDTH, default = PenWidth::Zero)]
    pub line_width: PenWidth,
    #[param(key = IS_SOLID, default = false)]
    pub is_solid: bool,
}

/// A multi-segment polyline (RECORD=6).
#[derive(FromParams, Debug)]
pub(crate) struct SchPolyline {
    #[param(flatten)]
    pub base: SchGraphicalBase,
    #[param(indexed_coords, count_key = LOCATION_COUNT, x_prefix = "X", y_prefix = "Y")]
    pub vertices: Vec<CoordPoint>,
    #[param(key = LINE_WIDTH, default = PenWidth::Zero)]
    pub line_width: PenWidth,
    #[param(key = LINE_STYLE, default = LineStyle::Solid)]
    pub line_style: LineStyle,
    #[param(key = LINE_STYLE_EXT, default = LineStyle::Solid)]
    pub line_style_ext: LineStyle,
    #[param(key = LINE_SHAPE_SIZE, default = PenWidth::Zero)]
    pub line_shape_size: PenWidth,
    #[param(key = START_LINE_SHAPE, default = LineShape::None)]
    pub start_line_shape: LineShape,
    #[param(key = END_LINE_SHAPE, default = LineShape::None)]
    pub end_line_shape: LineShape,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}

/// A closed polygon (RECORD=7).
#[derive(FromParams, Debug)]
pub(crate) struct SchPolygon {
    #[param(flatten)]
    pub base: SchGraphicalBase,
    #[param(indexed_coords, count_key = LOCATION_COUNT, x_prefix = "X", y_prefix = "Y")]
    pub vertices: Vec<CoordPoint>,
    #[param(key = LINE_WIDTH, default = PenWidth::Zero)]
    pub line_width: PenWidth,
    #[param(key = IS_SOLID, default = false)]
    pub is_solid: bool,
    #[param(key = TRANSPARENT, default = false)]
    pub transparent: bool,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}

/// A Bezier curve (RECORD=5).
#[derive(FromParams, Debug)]
pub(crate) struct SchBezier {
    #[param(flatten)]
    pub base: SchGraphicalBase,
    #[param(indexed_coords, count_key = LOCATION_COUNT, x_prefix = "X", y_prefix = "Y")]
    pub vertices: Vec<CoordPoint>,
    #[param(key = LINE_WIDTH, default = PenWidth::Zero)]
    pub line_width: PenWidth,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}

/// An embedded or linked image (RECORD=30).
#[derive(FromParams, Debug)]
pub(crate) struct SchImage {
    #[param(flatten)]
    pub base: SchGraphicalBase,
    #[param(coord_point, x_key = CORNER_X, x_frac = CORNER_X_FRAC, y_key = CORNER_Y, y_frac = CORNER_Y_FRAC)]
    pub corner: CoordPoint,
    #[param(key = ORIENTATION, default = RotationBy90::Rotate0)]
    pub orientation: RotationBy90,
    #[param(key = LINE_WIDTH, default = PenWidth::Zero)]
    pub line_width: PenWidth,
    #[param(key = IS_SOLID, default = false)]
    pub is_solid: bool,
    #[param(key = EMBED_IMAGE, default = false)]
    pub embed_image: bool,
    #[param(key = FILE_NAME, default = String::new())]
    pub file_name: String,
    #[param(key = KEEP_ASPECT, default = false)]
    pub keep_aspect: bool,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}

// ── Text and annotation records ───────────────────────────────────────────────

/// A text label (RECORD=4).
#[derive(FromParams, Debug)]
pub(crate) struct SchLabel {
    #[param(flatten)]
    pub base: SchGraphicalBase,
    #[param(key = TEXT, default = String::new())]
    pub text: String,
    #[param(key = FONT_ID, default = 1i32)]
    pub font_id: i32,
    #[param(key = JUSTIFICATION, default = TextJustification::BottomLeft)]
    pub justification: TextJustification,
    #[param(key = ORIENTATION, default = RotationBy90::Rotate0)]
    pub orientation: RotationBy90,
    #[param(key = IS_MIRRORED, default = false)]
    pub is_mirrored: bool,
    #[param(key = IS_HIDDEN, default = false)]
    pub is_hidden: bool,
    #[param(key = URL, default = String::new())]
    pub url: String,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}

/// A designator annotation (RECORD=34).
#[derive(FromParams, Debug)]
pub(crate) struct SchDesignator {
    #[param(flatten)]
    pub base: SchGraphicalBase,
    #[param(key = TEXT, default = String::from("*"))]
    pub text: String,
    #[param(key = NAME, default = String::from("Designator"))]
    pub name: String,
    #[param(key = FONT_ID, default = 1i32)]
    pub font_id: i32,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
    #[param(key = READ_ONLY_STATE, default = ParameterReadOnlyState::Name)]
    pub read_only_state: ParameterReadOnlyState,
    #[param(key = IS_HIDDEN, default = false)]
    pub is_hidden: bool,
    #[param(key = ORIENTATION, default = RotationBy90::Rotate0)]
    pub orientation: RotationBy90,
    #[param(key = IS_MIRRORED, default = false)]
    pub is_mirrored: bool,
    #[param(key = JUSTIFICATION, default = TextJustification::BottomLeft)]
    pub justification: TextJustification,
    #[param(key = NOT_AUTO_POSITION, default = false)]
    pub not_auto_position: bool,
}

/// A parameter annotation (RECORD=41).
///
/// Used for Comment, Value, and user-defined parameters. The `name` field
/// determines the parameter's role (e.g., `"Comment"`, `"Value"`).
#[derive(FromParams, Debug)]
pub(crate) struct SchParameter {
    #[param(flatten)]
    pub base: SchGraphicalBase,
    #[param(key = TEXT, default = String::from("*"))]
    pub text: String,
    #[param(key = NAME, default = String::from("Comment"))]
    pub name: String,
    #[param(key = FONT_ID, default = 1i32)]
    pub font_id: i32,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
    #[param(key = READ_ONLY_STATE, default = ParameterReadOnlyState::None)]
    pub read_only_state: ParameterReadOnlyState,
    #[param(key = IS_HIDDEN, default = false)]
    pub is_hidden: bool,
    #[param(key = ORIENTATION, default = RotationBy90::Rotate0)]
    pub orientation: RotationBy90,
    #[param(key = IS_MIRRORED, default = false)]
    pub is_mirrored: bool,
    #[param(key = JUSTIFICATION, default = TextJustification::BottomLeft)]
    pub justification: TextJustification,
    #[param(key = NOT_AUTO_POSITION, default = false)]
    pub not_auto_position: bool,
    #[param(key = SHOW_NAME, default = false)]
    pub show_name: bool,
    #[param(key = PARAM_TYPE, default = ParameterType::String)]
    pub param_type: ParameterType,
    #[param(key = DESCRIPTION, default = String::new())]
    pub description: String,
    #[param(key = NOT_ALLOW_LIBRARY_SYNCHRONIZE, default = false)]
    pub not_allow_library_synchronize: bool,
    #[param(key = NOT_ALLOW_DATABASE_SYNCHRONIZE, default = false)]
    pub not_allow_database_synchronize: bool,
    #[param(key = TEXT_HORZ_ANCHOR, default = TextHorzAnchor::None)]
    pub text_horz_anchor: TextHorzAnchor,
    #[param(key = TEXT_VERT_ANCHOR, default = TextVertAnchor::None)]
    pub text_vert_anchor: TextVertAnchor,
    #[param(key = IS_IMAGE_PARAMETER, default = false)]
    pub is_image_parameter: bool,
}

/// A text frame (bordered text box) (RECORD=28).
#[derive(FromParams, Debug)]
pub(crate) struct SchTextFrame {
    #[param(flatten)]
    pub base: SchGraphicalBase,
    #[param(coord_point, x_key = CORNER_X, x_frac = CORNER_X_FRAC, y_key = CORNER_Y, y_frac = CORNER_Y_FRAC)]
    pub corner: CoordPoint,
    #[param(key = TEXT, default = String::new())]
    pub text: String,
    #[param(key = FONT_ID, default = 1i32)]
    pub font_id: i32,
    #[param(key = ALIGNMENT, default = TextJustification::BottomLeft)]
    pub alignment: TextJustification,
    #[param(key = WORD_WRAP, default = false)]
    pub word_wrap: bool,
    #[param(key = IS_SOLID, default = false)]
    pub is_solid: bool,
    #[param(key = LINE_WIDTH, default = PenWidth::Small)]
    pub line_width: PenWidth,
    #[param(key = TEXT_COLOR, default = Color::BLACK)]
    pub text_color: Color,
    #[param(coord, key = TEXT_MARGIN, frac_key = TEXT_MARGIN_FRAC)]
    pub text_margin: Coord,
    #[param(key = SHOW_BORDER, default = true)]
    pub show_border: bool,
    #[param(key = TRANSPARENT, default = true)]
    pub transparent: bool,
    #[param(key = CLIP_TO_RECT, default = false)]
    pub clip_to_rect: bool,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}

// ── Implementation/model records ─────────────────────────────────────────────

/// Container for component implementation (footprint) entries (RECORD=44).
#[derive(FromParams, Debug)]
pub(crate) struct SchImplementationList {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
}

/// A single footprint/model assignment (RECORD=45).
///
/// Parsed from parameter stream. The datafile triplets (ModelDatafile{i},
/// ModelDatafileEntity{i}, ModelDatafileKind{i}) are stored as flat Strings
/// for index 0 only at this milestone; full multi-datafile support can be
/// added when assert_exhausted reveals more entries in real files.
#[derive(FromParams, Debug)]
pub(crate) struct SchImplementation {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(key = DESCRIPTION, default = String::new())]
    pub description: String,
    #[param(key = USE_COMPONENT_LIBRARY, default = false)]
    pub use_component_library: bool,
    #[param(key = MODEL_NAME, default = String::new())]
    pub model_name: String,
    #[param(key = MODEL_TYPE, default = String::new())]
    pub model_type: String,
    #[param(key = IS_CURRENT, default = false)]
    pub is_current: bool,
    #[param(key = DATALINKS_LOCKED, default = false)]
    pub datalinks_locked: bool,
    #[param(key = DATABASE_DATALINKS_LOCKED, default = false)]
    pub database_datalinks_locked: bool,
    #[param(key = INTEGRATED_MODEL, default = false)]
    pub integrated_model: bool,
    #[param(key = DATABASE_MODEL, default = false)]
    pub database_model: bool,
    #[param(key = MODEL_VAULT_GUID, default = String::new())]
    pub model_vault_guid: String,
    #[param(key = MODEL_ITEM_GUID, default = String::new())]
    pub model_item_guid: String,
    #[param(key = MODEL_REVISION_GUID, default = String::new())]
    pub model_revision_guid: String,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
    #[param(key = MODEL_LOCATION, default = String::new())]
    pub model_location: String,
    #[param(key = DATAFILE_COUNT, default = 0i32)]
    pub datafile_count: i32,
    #[param(key = "ModelDatafile0", default = String::new())]
    pub model_datafile0: String,
    #[param(key = "ModelDatafileEntity0", default = String::new())]
    pub model_datafile_entity0: String,
    #[param(key = "ModelDatafileKind0", default = String::new())]
    pub model_datafile_kind0: String,
}

/// Container for pin-to-pad mapping entries (RECORD=46).
#[derive(FromParams, Debug)]
pub(crate) struct SchImplementationMap {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
}

/// A single pin-to-pad designator mapping entry (RECORD=47).
///
/// The `des_intf` field holds the designator interface string (the pin name
/// that this mapping applies to). The indexed `DesImp{i}` values are stored
/// as a `Vec<String>`.
#[derive(Debug)]
pub(crate) struct SchMapDefiner {
    pub base: SchPrimitiveBase,
    pub des_intf: String,
    pub des_imps: Vec<String>,
}

impl SchMapDefiner {
    pub fn from_params(params: &mut ParameterCollection) -> crate::Result<Self> {
        let base = SchPrimitiveBase::from_params(params)?;
        let des_intf: String = params.remove_with_default(DES_INTF, String::new())?;
        let des_imp_count: i32 = params.remove_with_default(DES_IMP_COUNT, 0i32)?;
        let mut des_imps = Vec::with_capacity(des_imp_count as usize);
        for i in 0..des_imp_count {
            let key = format!("DesImp{i}");
            let v: String = params.remove_with_default(&key, String::new())?;
            des_imps.push(v);
        }
        // Consume any additional DesImp{i} entries not covered by DesImpCount (e.g., when
        // DesImpCount is absent or zero but entries are still present in older files).
        let mut extra_i = des_imp_count as usize;
        loop {
            let key = format!("DesImp{extra_i}");
            match params.remove_optional::<String>(&key)? {
                Some(v) => des_imps.push(v),
                None => break,
            }
            extra_i += 1;
        }
        Ok(SchMapDefiner { base, des_intf, des_imps })
    }
}

/// A parameter list container (RECORD=48).
#[derive(FromParams, Debug)]
pub(crate) struct SchParameterList {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
}

// ── SchRecord dispatch enum ───────────────────────────────────────────────────

/// All implemented schematic record variants.
///
/// Variants are added incrementally as record types are implemented.
pub(crate) enum SchRecord {
    Component(SchComponent),
    Pin(SchPin),
    Line(SchLine),
    Rectangle(SchRectangle),
    RoundRectangle(SchRoundRectangle),
    Arc(SchArc),
    EllipticalArc(SchEllipticalArc),
    Ellipse(SchEllipse),
    Pie(SchPie),
    Polyline(SchPolyline),
    Polygon(SchPolygon),
    Bezier(SchBezier),
    Image(SchImage),
    Label(SchLabel),
    Designator(SchDesignator),
    Parameter(SchParameter),
    TextFrame(SchTextFrame),
    ImplementationList(SchImplementationList),
    Implementation(SchImplementation),
    ImplementationMap(SchImplementationMap),
    MapDefiner(SchMapDefiner),
    ParameterList(SchParameterList),
}

// ── SchLibComponent ───────────────────────────────────────────────────────────

/// A single component entry in a SchLib file, holding its component record and
/// all child records (pins, graphical primitives, text annotations, etc.).
pub(crate) struct SchLibComponent {
    pub component: SchComponent,
    pub records: Vec<SchRecord>,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use altium_format_types::{IeeeSymbol, PinElectricalType, RotationBy90, PenWidth, LineStyle, TextJustification, ParameterReadOnlyState, ParameterType};

    use super::*;

    fn pc(s: &str) -> ParameterCollection {
        let mut data = s.to_owned();
        data.push('\0');
        ParameterCollection::from_bytes(data.as_bytes()).unwrap()
    }

    // ── SchPrimitiveBase tests ────────────────────────────────────────────────

    #[test]
    fn primitive_base_all_defaults() {
        let mut params = pc("|");
        let base = SchPrimitiveBase::from_params(&mut params).unwrap();
        assert_eq!(base.owner_index, -1);
        assert!(!base.is_not_accessible);
        assert_eq!(base.owner_part_id, -1);
        assert_eq!(base.owner_part_display_mode, 0);
        assert!(!base.graphically_locked);
        assert_eq!(base.index_in_sheet, -1);
    }

    #[test]
    fn primitive_base_explicit_fields() {
        let mut params = pc("|OwnerIndex=3|IsNotAccesible=T|OwnerPartId=2|OwnerPartDisplayMode=1|GraphicallyLocked=T|IndexInSheet=5|");
        let base = SchPrimitiveBase::from_params(&mut params).unwrap();
        assert_eq!(base.owner_index, 3);
        assert!(base.is_not_accessible);
        assert_eq!(base.owner_part_id, 2);
        assert_eq!(base.owner_part_display_mode, 1);
        assert!(base.graphically_locked);
        assert_eq!(base.index_in_sheet, 5);
    }

    // ── SchGraphicalBase tests ────────────────────────────────────────────────

    #[test]
    fn graphical_base_flattens_primitive() {
        let mut params = pc("|OwnerIndex=7|Location.X=10|Location.Y=20|");
        let base = SchGraphicalBase::from_params(&mut params).unwrap();
        assert_eq!(base.primitive.owner_index, 7);
        assert_eq!(base.location.x.to_internal(), 10 * 100_000);
        assert_eq!(base.location.y.to_internal(), 20 * 100_000);
    }

    #[test]
    fn graphical_base_fractional_coords() {
        let mut params = pc("|Location.X=100|Location.X_Frac=50000|Location.Y=200|Location.Y_Frac=75000|");
        let base = SchGraphicalBase::from_params(&mut params).unwrap();
        // 100 * 100_000 + 50_000 = 10_050_000
        assert_eq!(base.location.x.to_internal(), 10_050_000);
        // 200 * 100_000 + 75_000 = 20_075_000
        assert_eq!(base.location.y.to_internal(), 20_075_000);
    }

    #[test]
    fn graphical_base_default_colors() {
        let mut params = pc("|Location.X=0|Location.Y=0|");
        let base = SchGraphicalBase::from_params(&mut params).unwrap();
        assert_eq!(base.color, Color::BLACK);
        assert_eq!(base.area_color, Color::BLACK);
    }

    // ── Binary pin helpers ────────────────────────────────────────────────────

    /// Builds a valid binary pin payload for testing.
    #[allow(clippy::too_many_arguments)]
    fn make_pin_bytes(
        owner_index: i32,
        owner_part_id: i16,
        owner_part_display_mode: u8,
        symbol_inner_edge: u8,
        symbol_outer_edge: u8,
        symbol_inside: u8,
        symbol_outside: u8,
        description: &[u8],
        formal_type: u8,
        electrical: u8,
        conglomerate: u8,
        pin_length: i16,
        location_x: i16,
        location_y: i16,
        color: i32,
        name: &[u8],
        designator: &[u8],
        swap_id_pin: &[u8],
        swap_id_part: &[u8],
        default_value: &[u8],
    ) -> Vec<u8> {
        let mut v = Vec::new();
        v.push(0x02); // binary_code
        v.extend_from_slice(&owner_index.to_le_bytes());
        v.extend_from_slice(&owner_part_id.to_le_bytes());
        v.push(owner_part_display_mode);
        v.push(symbol_inner_edge);
        v.push(symbol_outer_edge);
        v.push(symbol_inside);
        v.push(symbol_outside);
        v.push(description.len() as u8);
        v.extend_from_slice(description);
        v.push(formal_type);
        v.push(electrical);
        v.push(conglomerate);
        v.extend_from_slice(&pin_length.to_le_bytes());
        v.extend_from_slice(&location_x.to_le_bytes());
        v.extend_from_slice(&location_y.to_le_bytes());
        v.extend_from_slice(&color.to_le_bytes());
        v.push(name.len() as u8);
        v.extend_from_slice(name);
        v.push(designator.len() as u8);
        v.extend_from_slice(designator);
        v.push(swap_id_pin.len() as u8);
        v.extend_from_slice(swap_id_pin);
        v.push(swap_id_part.len() as u8);
        v.extend_from_slice(swap_id_part);
        v.push(default_value.len() as u8);
        v.extend_from_slice(default_value);
        v
    }

    // ── Binary pin parsing tests ──────────────────────────────────────────────

    #[test]
    fn parse_binary_pin_minimal() {
        let data = make_pin_bytes(
            0, 1, 0,      // owner_index, owner_part_id, owner_part_display_mode
            0, 0, 0, 0,   // no symbols
            b"",          // empty description
            0,            // formal_type
            4,            // Passive
            0x00,         // conglomerate: Rotate0, no flags
            3,            // pin_length = 3 DXP units
            10,           // location_x = 10
            20,           // location_y = 20
            0x0000FF00i32, // green in BGR
            b"A1",        // name
            b"PA0",       // designator
            b"", b"", b"", // swap_id_pin, swap_id_part, default_value
        );
        let pin = parse_binary_pin(&data).unwrap();
        assert_eq!(pin.symbol_inner_edge, IeeeSymbol::NoSymbol);
        assert_eq!(pin.electrical, PinElectricalType::Passive);
        assert_eq!(pin.orientation, RotationBy90::Rotate0);
        assert!(!pin.is_hidden);
        assert!(!pin.show_name);
        assert!(!pin.show_designator);
        assert!(!pin.is_not_accessible);
        assert_eq!(pin.pin_length.to_internal(), 3 * 100_000);
        assert_eq!(pin.location.x.to_internal(), 10 * 100_000);
        assert_eq!(pin.location.y.to_internal(), 20 * 100_000);
        assert_eq!(pin.name, "A1");
        assert_eq!(pin.designator, "PA0");
    }

    #[test]
    fn parse_binary_pin_empty_strings() {
        let data = make_pin_bytes(
            0, 1, 0, 0, 0, 0, 0, b"", 0, 4, 0x00, 1, 0, 0, 0, b"", b"", b"", b"", b"",
        );
        let pin = parse_binary_pin(&data).unwrap();
        assert!(pin.description.is_empty());
        assert!(pin.name.is_empty());
        assert!(pin.designator.is_empty());
    }

    #[test]
    fn parse_binary_pin_with_description() {
        let desc = b"This is a pin description";
        let data = make_pin_bytes(
            0, 1, 0, 0, 0, 0, 0, desc, 0, 4, 0x00, 1, 5, 5, 0, b"VCC", b"1", b"", b"", b"",
        );
        let pin = parse_binary_pin(&data).unwrap();
        assert_eq!(pin.description, "This is a pin description");
        assert_eq!(pin.name, "VCC");
        assert_eq!(pin.designator, "1");
    }

    #[test]
    fn parse_binary_pin_conglomerate_all_flags() {
        // All bits set: Rotate270 (0x03) | IsHidden | ShowName | ShowDesignator
        //             | NotAccessible | GraphicallyLocked | OwnerIndexAdditionalList
        let cong: u8 = 0x03 | 0x04 | 0x08 | 0x10 | 0x20 | 0x40 | 0x80;
        let data = make_pin_bytes(0, 1, 0, 0, 0, 0, 0, b"", 0, 4, cong, 1, 0, 0, 0, b"", b"", b"", b"", b"");
        let pin = parse_binary_pin(&data).unwrap();
        assert_eq!(pin.orientation, RotationBy90::Rotate270);
        assert!(pin.is_hidden);
        assert!(pin.show_name);
        assert!(pin.show_designator);
        assert!(pin.is_not_accessible);
        assert!(pin.graphically_locked);
        assert!(pin.owner_index_additional_list);
    }

    #[test]
    fn parse_binary_pin_orientation_variants() {
        for (raw, expected) in [
            (0u8, RotationBy90::Rotate0),
            (1u8, RotationBy90::Rotate90),
            (2u8, RotationBy90::Rotate180),
            (3u8, RotationBy90::Rotate270),
        ] {
            let data = make_pin_bytes(0, 1, 0, 0, 0, 0, 0, b"", 0, 4, raw, 1, 0, 0, 0, b"", b"", b"", b"", b"");
            let pin = parse_binary_pin(&data).unwrap();
            assert_eq!(pin.orientation, expected, "orientation for conglomerate {raw}");
        }
    }

    #[test]
    fn parse_binary_pin_coord_conversion() {
        // pin_length=5, x=100, y=-50 in DXP units
        let data = make_pin_bytes(0, 1, 0, 0, 0, 0, 0, b"", 0, 4, 0, 5, 100, -50, 0, b"", b"", b"", b"", b"");
        let pin = parse_binary_pin(&data).unwrap();
        assert_eq!(pin.pin_length.to_internal(), 5 * 100_000);
        assert_eq!(pin.location.x.to_internal(), 100 * 100_000);
        assert_eq!(pin.location.y.to_internal(), -50 * 100_000);
    }

    #[test]
    fn parse_binary_pin_all_ieee_symbols() {
        for sym in 0u8..=36 {
            let data = make_pin_bytes(0, 1, 0, sym, sym, sym, sym, b"", 0, 4, 0, 1, 0, 0, 0, b"", b"", b"", b"", b"");
            let pin = parse_binary_pin(&data).unwrap();
            let expected = IeeeSymbol::try_from(sym).unwrap();
            assert_eq!(pin.symbol_inner_edge, expected);
            assert_eq!(pin.symbol_outer_edge, expected);
            assert_eq!(pin.symbol_inside, expected);
            assert_eq!(pin.symbol_outside, expected);
        }
    }

    #[test]
    fn parse_binary_pin_all_electrical_types() {
        for elec in 0u8..=7 {
            let data = make_pin_bytes(0, 1, 0, 0, 0, 0, 0, b"", 0, elec, 0, 1, 0, 0, 0, b"", b"", b"", b"", b"");
            let pin = parse_binary_pin(&data).unwrap();
            let expected = PinElectricalType::try_from(elec).unwrap();
            assert_eq!(pin.electrical, expected);
        }
    }

    #[test]
    fn parse_binary_pin_invalid_binary_code() {
        let mut data = make_pin_bytes(0, 1, 0, 0, 0, 0, 0, b"", 0, 4, 0, 1, 0, 0, 0, b"", b"", b"", b"", b"");
        data[0] = 0x01; // wrong code
        let err = parse_binary_pin(&data).unwrap_err();
        assert!(
            matches!(err, AltiumFormatError::UnknownBinaryCode(0x01)),
            "expected UnknownBinaryCode(0x01), got {err:?}"
        );
    }

    #[test]
    fn parse_binary_pin_truncated_data() {
        // Only header byte — not enough to read
        let data = [0x02u8];
        let err = parse_binary_pin(&data).unwrap_err();
        assert!(
            matches!(err, AltiumFormatError::BinaryReadPastEnd { .. }),
            "expected BinaryReadPastEnd, got {err:?}"
        );
    }

    #[test]
    fn parse_binary_pin_trailing_bytes() {
        let mut data = make_pin_bytes(0, 1, 0, 0, 0, 0, 0, b"", 0, 4, 0, 1, 0, 0, 0, b"", b"", b"", b"", b"");
        data.push(0xFF); // extra byte
        let err = parse_binary_pin(&data).unwrap_err();
        assert!(
            matches!(err, AltiumFormatError::UnexpectedTrailingData { .. }),
            "expected UnexpectedTrailingData, got {err:?}"
        );
    }

    #[test]
    fn decode_pin_conglomerate_individual_flags() {
        let f = decode_pin_conglomerate(0x04).unwrap();
        assert!(f.is_hidden);
        assert!(!f.show_name);

        let f = decode_pin_conglomerate(0x08).unwrap();
        assert!(f.show_name);
        assert!(!f.is_hidden);

        let f = decode_pin_conglomerate(0x10).unwrap();
        assert!(f.show_designator);

        let f = decode_pin_conglomerate(0x20).unwrap();
        assert!(f.is_not_accessible);

        let f = decode_pin_conglomerate(0x40).unwrap();
        assert!(f.graphically_locked);

        let f = decode_pin_conglomerate(0x80).unwrap();
        assert!(f.owner_index_additional_list);
    }

    #[test]
    fn sidecar_fields_initialized_to_defaults() {
        let data = make_pin_bytes(0, 1, 0, 0, 0, 0, 0, b"", 0, 4, 0, 1, 0, 0, 0, b"", b"", b"", b"", b"");
        let pin = parse_binary_pin(&data).unwrap();
        assert_eq!(pin.owner_index, 0);
        assert_eq!(pin.owner_part_id, 1);
        assert!(pin.swap_id_pin.is_empty());
        assert!(pin.swap_id_part.is_empty());
        assert!(pin.default_value.is_empty());
        assert_eq!(pin.pin_symbol_line_width, 0);
        assert!(pin.pin_package_length.is_empty());
        assert!(pin.propagation_delay.is_empty());
        assert!(pin.selected_functions.is_empty());
        assert!(pin.defined_functions.is_empty());
        assert!(pin.name_text_data.is_none());
        assert!(pin.designator_text_data.is_none());
    }

    // ── Graphical primitive tests ─────────────────────────────────────────────

    #[test]
    fn line_parsed() {
        let mut params = pc("|Location.X=10|Location.Y=20|Corner.X=30|Corner.Y=40|LineWidth=1|LineStyle=2|");
        let line = SchLine::from_params(&mut params).unwrap();
        assert_eq!(line.base.location.x.to_internal(), 10 * 100_000);
        assert_eq!(line.base.location.y.to_internal(), 20 * 100_000);
        assert_eq!(line.corner.x.to_internal(), 30 * 100_000);
        assert_eq!(line.corner.y.to_internal(), 40 * 100_000);
        assert_eq!(line.line_width, PenWidth::Small);
        assert_eq!(line.line_style, LineStyle::Dotted);
    }

    #[test]
    fn line_defaults() {
        let mut params = pc("|Location.X=0|Location.Y=0|Corner.X=0|Corner.Y=0|");
        let line = SchLine::from_params(&mut params).unwrap();
        assert_eq!(line.line_width, PenWidth::Zero);
        assert_eq!(line.line_style, LineStyle::Solid);
    }

    #[test]
    fn rectangle_parsed() {
        let mut params = pc("|Location.X=0|Location.Y=0|Corner.X=50|Corner.Y=60|LineWidth=2|LineStyleExt=1|IsSolid=T|Transparent=F|");
        let rect = SchRectangle::from_params(&mut params).unwrap();
        assert_eq!(rect.corner.x.to_internal(), 50 * 100_000);
        assert_eq!(rect.corner.y.to_internal(), 60 * 100_000);
        assert_eq!(rect.line_width, PenWidth::Medium);
        assert_eq!(rect.line_style, LineStyle::Dashed);
        assert!(rect.is_solid);
        assert!(!rect.transparent);
    }

    #[test]
    fn arc_parsed() {
        let mut params = pc("|Location.X=5|Location.Y=5|Radius=100|StartAngle=45|EndAngle=270|LineWidth=1|");
        let arc = SchArc::from_params(&mut params).unwrap();
        assert_eq!(arc.radius.to_internal(), 100 * 100_000);
        assert!((arc.start_angle - 45.0).abs() < f64::EPSILON);
        assert!((arc.end_angle - 270.0).abs() < f64::EPSILON);
        assert_eq!(arc.line_width, PenWidth::Small);
    }

    #[test]
    fn arc_with_frac_radius() {
        let mut params = pc("|Location.X=0|Location.Y=0|Radius=10|Radius_Frac=50000|StartAngle=0|EndAngle=360|LineWidth=0|");
        let arc = SchArc::from_params(&mut params).unwrap();
        assert_eq!(arc.radius.to_internal(), 10 * 100_000 + 50_000);
    }

    #[test]
    fn elliptical_arc_parsed() {
        let mut params = pc("|Location.X=0|Location.Y=0|Radius=50|SecondaryRadius=25|StartAngle=0|EndAngle=180|LineWidth=0|");
        let ea = SchEllipticalArc::from_params(&mut params).unwrap();
        assert_eq!(ea.radius.to_internal(), 50 * 100_000);
        assert_eq!(ea.secondary_radius.to_internal(), 25 * 100_000);
        assert!((ea.start_angle - 0.0).abs() < f64::EPSILON);
        assert!((ea.end_angle - 180.0).abs() < f64::EPSILON);
    }

    #[test]
    fn ellipse_parsed() {
        let mut params = pc("|Location.X=0|Location.Y=0|Radius=30|SecondaryRadius=20|LineWidth=0|IsSolid=T|Transparent=F|");
        let el = SchEllipse::from_params(&mut params).unwrap();
        assert_eq!(el.radius.to_internal(), 30 * 100_000);
        assert_eq!(el.secondary_radius.to_internal(), 20 * 100_000);
        assert!(el.is_solid);
        assert!(!el.transparent);
    }

    #[test]
    fn pie_parsed() {
        let mut params = pc("|Location.X=0|Location.Y=0|Radius=40|SecondaryRadius=40|StartAngle=30|EndAngle=150|LineWidth=0|IsSolid=T|Transparent=F|");
        let pie = SchPie::from_params(&mut params).unwrap();
        assert_eq!(pie.radius.to_internal(), 40 * 100_000);
        assert!((pie.start_angle - 30.0).abs() < f64::EPSILON);
        assert!((pie.end_angle - 150.0).abs() < f64::EPSILON);
        assert!(pie.is_solid);
    }

    #[test]
    fn polyline_parsed() {
        let mut params = pc("|Location.X=0|Location.Y=0|LocationCount=3|X1=1|Y1=2|X2=3|Y2=4|X3=5|Y3=6|LineWidth=1|LineStyle=0|LineShapeSize=0|StartLineShape=0|EndLineShape=0|");
        let pl = SchPolyline::from_params(&mut params).unwrap();
        assert_eq!(pl.vertices.len(), 3);
        assert_eq!(pl.vertices[0].x.to_internal(), 1 * 100_000);
        assert_eq!(pl.vertices[0].y.to_internal(), 2 * 100_000);
        assert_eq!(pl.vertices[2].x.to_internal(), 5 * 100_000);
        assert_eq!(pl.vertices[2].y.to_internal(), 6 * 100_000);
        assert_eq!(pl.line_width, PenWidth::Small);
    }

    #[test]
    fn polygon_parsed() {
        let mut params = pc("|Location.X=0|Location.Y=0|LocationCount=2|X1=10|Y1=20|X2=30|Y2=40|LineWidth=0|LineStyle=0|IsSolid=T|Transparent=F|");
        let pg = SchPolygon::from_params(&mut params).unwrap();
        assert_eq!(pg.vertices.len(), 2);
        assert_eq!(pg.vertices[1].x.to_internal(), 30 * 100_000);
        assert!(pg.is_solid);
    }

    #[test]
    fn bezier_parsed() {
        let mut params = pc("|Location.X=0|Location.Y=0|LocationCount=4|X1=0|Y1=0|X2=10|Y2=20|X3=30|Y3=20|X4=40|Y4=0|LineWidth=1|LineStyle=0|");
        let bz = SchBezier::from_params(&mut params).unwrap();
        assert_eq!(bz.vertices.len(), 4);
        assert_eq!(bz.vertices[3].x.to_internal(), 40 * 100_000);
        assert_eq!(bz.line_width, PenWidth::Small);
    }

    #[test]
    fn image_parsed() {
        let mut params = pc("|Location.X=0|Location.Y=0|Corner.X=100|Corner.Y=80|EmbedImage=T|FileName=test.bmp|KeepAspect=T|");
        let img = SchImage::from_params(&mut params).unwrap();
        assert_eq!(img.corner.x.to_internal(), 100 * 100_000);
        assert_eq!(img.corner.y.to_internal(), 80 * 100_000);
        assert!(img.embed_image);
        assert_eq!(img.file_name, "test.bmp");
        assert!(img.keep_aspect);
    }

    #[test]
    fn image_defaults() {
        let mut params = pc("|Location.X=0|Location.Y=0|Corner.X=0|Corner.Y=0|");
        let img = SchImage::from_params(&mut params).unwrap();
        assert!(!img.embed_image);
        assert!(img.file_name.is_empty());
        assert!(!img.keep_aspect);
    }

    // ── Text and annotation record tests ──────────────────────────────────────

    #[test]
    fn label_parsed() {
        let mut params = pc("|Location.X=10|Location.Y=20|Text=Hello|FontID=2|Justification=3|Orientation=1|IsMirrored=T|IsHidden=F|");
        let label = SchLabel::from_params(&mut params).unwrap();
        assert_eq!(label.base.location.x.to_internal(), 10 * 100_000);
        assert_eq!(label.text, "Hello");
        assert_eq!(label.font_id, 2);
        assert_eq!(label.justification, TextJustification::CenterLeft);
        assert_eq!(label.orientation, RotationBy90::Rotate90);
        assert!(label.is_mirrored);
        assert!(!label.is_hidden);
    }

    #[test]
    fn label_defaults() {
        let mut params = pc("|Location.X=0|Location.Y=0|");
        let label = SchLabel::from_params(&mut params).unwrap();
        assert!(label.text.is_empty());
        assert_eq!(label.font_id, 1);
        assert_eq!(label.justification, TextJustification::BottomLeft);
        assert_eq!(label.orientation, RotationBy90::Rotate0);
        assert!(!label.is_mirrored);
        assert!(!label.is_hidden);
    }

    #[test]
    fn label_hidden() {
        let mut params = pc("|Location.X=0|Location.Y=0|IsHidden=T|");
        let label = SchLabel::from_params(&mut params).unwrap();
        assert!(label.is_hidden);
    }

    #[test]
    fn designator_parsed() {
        let mut params = pc("|Location.X=5|Location.Y=5|Text=U1|Name=Designator|FontID=1|UniqueID=ABCDEF|ReadOnlyState=1|IsHidden=F|Orientation=0|IsMirrored=F|Justification=0|NotAutoPosition=F|");
        let des = SchDesignator::from_params(&mut params).unwrap();
        assert_eq!(des.text, "U1");
        assert_eq!(des.name, "Designator");
        assert_eq!(des.font_id, 1);
        assert_eq!(des.unique_id, "ABCDEF");
        assert_eq!(des.read_only_state, ParameterReadOnlyState::Name);
        assert!(!des.is_hidden);
        assert!(!des.not_auto_position);
    }

    #[test]
    fn designator_defaults() {
        let mut params = pc("|Location.X=0|Location.Y=0|");
        let des = SchDesignator::from_params(&mut params).unwrap();
        assert_eq!(des.text, "*");
        assert_eq!(des.name, "Designator");
        assert_eq!(des.read_only_state, ParameterReadOnlyState::Name);
        assert!(!des.is_hidden);
        assert!(!des.not_auto_position);
    }

    #[test]
    fn parameter_comment_parsed() {
        let mut params = pc("|Location.X=0|Location.Y=0|Text=100nF|Name=Comment|FontID=1|UniqueID=XYZ|ReadOnlyState=0|IsHidden=F|Orientation=0|IsMirrored=F|Justification=0|NotAutoPosition=F|ShowName=T|ParamType=0|");
        let param = SchParameter::from_params(&mut params).unwrap();
        assert_eq!(param.text, "100nF");
        assert_eq!(param.name, "Comment");
        assert!(param.show_name);
        assert_eq!(param.param_type, ParameterType::String);
    }

    #[test]
    fn parameter_defaults() {
        let mut params = pc("|Location.X=0|Location.Y=0|");
        let param = SchParameter::from_params(&mut params).unwrap();
        assert_eq!(param.text, "*");
        assert_eq!(param.name, "Comment");
        assert_eq!(param.read_only_state, ParameterReadOnlyState::None);
        assert!(!param.show_name);
    }

    #[test]
    fn parameter_hidden() {
        let mut params = pc("|Location.X=0|Location.Y=0|IsHidden=T|");
        let param = SchParameter::from_params(&mut params).unwrap();
        assert!(param.is_hidden);
    }

    #[test]
    fn text_frame_parsed() {
        let mut params = pc("|Location.X=0|Location.Y=0|Corner.X=200|Corner.Y=100|Text=Hello World|FontID=2|Alignment=1|WordWrap=T|IsSolid=F|LineWidth=1|TextMargin=5|ShowBorder=T|Transparent=T|ClipToRect=F|");
        let tf = SchTextFrame::from_params(&mut params).unwrap();
        assert_eq!(tf.corner.x.to_internal(), 200 * 100_000);
        assert_eq!(tf.text, "Hello World");
        assert_eq!(tf.font_id, 2);
        assert_eq!(tf.alignment, TextJustification::BottomCenter);
        assert!(tf.word_wrap);
        assert!(!tf.is_solid);
        assert_eq!(tf.line_width, PenWidth::Small);
        assert_eq!(tf.text_margin.to_internal(), 5 * 100_000);
        assert!(tf.show_border);
        assert!(tf.transparent);
        assert!(!tf.clip_to_rect);
    }

    #[test]
    fn text_frame_defaults() {
        let mut params = pc("|Location.X=0|Location.Y=0|Corner.X=0|Corner.Y=0|");
        let tf = SchTextFrame::from_params(&mut params).unwrap();
        assert!(tf.text.is_empty());
        assert_eq!(tf.font_id, 1);
        assert_eq!(tf.alignment, TextJustification::BottomLeft);
        assert!(!tf.word_wrap);
        assert_eq!(tf.line_width, PenWidth::Small);
        assert!(tf.show_border);
        assert!(tf.transparent);
    }

    // ── Implementation/model record tests ────────────────────────────────────

    #[test]
    fn implementation_list_parses_primitive_base() {
        let mut params = pc("|OwnerIndex=0|");
        let il = SchImplementationList::from_params(&mut params).unwrap();
        assert_eq!(il.base.owner_index, 0);
    }

    #[test]
    fn implementation_parsed() {
        let mut params = pc("|ModelName=SOIC127P600X175-8N|ModelType=PCBLIB|Description=SOP-8|IsCurrent=T|DatalinksLocked=F|DatabaseDatalinksLocked=F|IntegratedModel=F|DatabaseModel=F|UniqueID=ABC123|DatafileCount=1|ModelDatafile0=Lib.PcbLib|ModelDatafileEntity0=SOIC127P600X175-8N|ModelDatafileKind0=PCBLIB|UseComponentLibrary=F|");
        let imp = SchImplementation::from_params(&mut params).unwrap();
        assert_eq!(imp.model_name, "SOIC127P600X175-8N");
        assert_eq!(imp.model_type, "PCBLIB");
        assert_eq!(imp.description, "SOP-8");
        assert!(imp.is_current);
        assert!(!imp.datalinks_locked);
        assert!(!imp.integrated_model);
        assert_eq!(imp.unique_id, "ABC123");
        assert_eq!(imp.datafile_count, 1);
        assert_eq!(imp.model_datafile0, "Lib.PcbLib");
        assert_eq!(imp.model_datafile_entity0, "SOIC127P600X175-8N");
        assert_eq!(imp.model_datafile_kind0, "PCBLIB");
    }

    #[test]
    fn implementation_defaults() {
        let mut params = pc("|");
        let imp = SchImplementation::from_params(&mut params).unwrap();
        assert!(imp.model_name.is_empty());
        assert!(imp.model_type.is_empty());
        assert!(!imp.is_current);
        assert!(!imp.integrated_model);
        assert!(!imp.database_model);
        assert_eq!(imp.datafile_count, 0);
    }

    #[test]
    fn implementation_map_parses() {
        let mut params = pc("|OwnerIndex=3|");
        let im = SchImplementationMap::from_params(&mut params).unwrap();
        assert_eq!(im.base.owner_index, 3);
    }

    #[test]
    fn map_definer_parsed() {
        let mut params = pc("|DesIntf=PA0|DesImpCount=1|DesImp0=pad1|");
        let md = SchMapDefiner::from_params(&mut params).unwrap();
        assert_eq!(md.des_intf, "PA0");
        assert_eq!(md.des_imps.len(), 1);
        assert_eq!(md.des_imps[0], "pad1");
    }

    #[test]
    fn map_definer_defaults() {
        let mut params = pc("|");
        let md = SchMapDefiner::from_params(&mut params).unwrap();
        assert!(md.des_intf.is_empty());
        assert!(md.des_imps.is_empty());
    }

    #[test]
    fn parameter_list_parses_primitive_base() {
        let mut params = pc("|OwnerIndex=1|");
        let pl = SchParameterList::from_params(&mut params).unwrap();
        assert_eq!(pl.base.owner_index, 1);
    }
}
