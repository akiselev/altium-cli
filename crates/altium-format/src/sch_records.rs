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

use altium_format_derive::{FromParams, ToParams};
use altium_format_types::{
    Color, ComponentKind, Coord, CoordPoint, HorizontalAlign, IeeeSymbol, LeftRightSide, LineShape,
    LineStyle, ParameterReadOnlyState, ParameterType, PenWidth, PinElectricalType, RotationBy90,
    SchDisplaySettings, SchRecordType, SheetBorderStyle, SheetOrientation,
    SheetReferenceZoneStyle, SheetStyle, SheetSymbolType, StdLogicState, TextHorzAnchor,
    TextJustification, TextVertAnchor,
    constants::{
        component::{
            ALIAS_LIST, ALL_PIN_COUNT, COMPONENT_DESCRIPTION, COMPONENT_KIND,
            COMPONENT_KIND_VERSION2, COMPONENT_KIND_VERSION3, CURRENT_PART_ID, DESIGNATOR_LOCKED,
            DISPLAY_FIELD_NAMES, DISPLAY_MODE, DISPLAY_MODE_COUNT, HAS_ONLY_CURRENT_PART_INFO,
            IS_MIRRORED, KEY_COMPONENT_UNIQUE_ID, LIB_REFERENCE, NOT_USE_LIBRARY_NAME, PART_COUNT,
            PART_ID_LOCKED, PINS_MOVEABLE, SHEET_PART_FILE_NAME, SHOW_HIDDEN_FIELDS,
        },
        electrical::{
            CONNECTION_PAIRS_TO_SUPPRESS, ELECTRICAL, ERROR_KIND_SET_TO_SUPPRESS, FORMAL_TYPE,
            IO_TYPE, IS_CROSS_SHEET_CONNECTOR, PORT_NAME_IS_HIDDEN, SHOW_NET_NAME, SIDE,
            SUPPRESS_ALL, SYMBOL_TYPE,
        },
        harness::{HARNESS_CONNECTOR_SIDE, HARNESS_TYPE, OBJECT_DEFINITION_ID, PRIMARY_CONNECTION_POSITION},
        locking::{
            GRAPHICALLY_LOCKED, IS_ACTIVE, IS_CURRENT, IS_HIDDEN, IS_NOT_ACCESSIBLE, LOCKED,
            NOT_AUTO_POSITION, OVERRIDE_NOT_AUTO_POSITION, READ_ONLY_STATE, SELECTION,
        },
        model::{
            DATABASE_DATALINKS_LOCKED, DATABASE_MODEL, DATAFILE_COUNT, DATALINKS_LOCKED,
            DES_IMP_COUNT, DES_INTF, INTEGRATED_MODEL, MODEL_ITEM_GUID, MODEL_LOCATION, MODEL_NAME,
            MODEL_REVISION_GUID, MODEL_TYPE, MODEL_VAULT_GUID, USE_COMPONENT_LIBRARY,
        },
        parsing::{C_BASE_UNIT, C_MAX_SHORT_STRING_LENGTH},
        pin::{
            DEF_VALUE, DESIGNATOR_CUSTOM_COLOR, DESIGNATOR_CUSTOM_FONT_ID,
            DESIGNATOR_CUSTOM_POSITION_MARGIN, NAME_CUSTOM_COLOR, NAME_CUSTOM_FONT_ID,
            NAME_CUSTOM_POSITION_MARGIN, PIN_BINARY_CODE, PIN_COLOR, PIN_CONGLOMERATE,
            PIN_CONGLOMERATE_GRAPHICALLY_LOCKED, PIN_CONGLOMERATE_IS_HIDDEN,
            PIN_CONGLOMERATE_NOT_ACCESSIBLE, PIN_CONGLOMERATE_ORIENTATION_MASK,
            PIN_CONGLOMERATE_OWNER_INDEX_ADDITIONAL_LIST, PIN_CONGLOMERATE_SHOW_DESIGNATOR,
            PIN_CONGLOMERATE_SHOW_NAME, PIN_DEFINED_FUNCTION, PIN_DEFINED_FUNCTIONS_COUNT,
            PIN_DESIGNATOR_POSITION_CONGLOMERATE, PIN_LENGTH, PIN_NAME_POSITION_CONGLOMERATE,
            PIN_PACKAGE_LENGTH as PIN_PACKAGE_LENGTH_KEY,
            PIN_PROPAGATION_DELAY as PIN_PROPAGATION_DELAY_KEY, PIN_SELECTED_FUNCTION,
            PIN_SELECTED_FUNCTIONS_COUNT, SWAP_ID_PAIR, SWAP_ID_PART, SWAP_ID_PIN, SYMBOL,
            SYMBOL_INNER, SYMBOL_INNER_EDGE, SYMBOL_LINE_WIDTH, SYMBOL_OUTER, SYMBOL_OUTER_EDGE,
        },
        record_structure::{
            ASSIGNED_INTERFACE, ASSIGNED_INTERFACE_SIGNAL, COLLAPSED, DISTANCE_FROM_TOP,
            INDEX_IN_SHEET, IS_IMAGE_PARAMETER, OWNER_INDEX, OWNER_PART_DISPLAY_MODE,
            OWNER_PART_ID, PARAM_TYPE, RECORD, RECORD_EX, SELECTION_MEMORY, UNION_INDEX,
            UNIQUE_ID, URL,
        },
        sheet::{
            AREA_COLOR, AUTHOR, BORDER_ON, CUSTOM_MARGIN_WIDTH, CUSTOM_X, CUSTOM_X_FRAC,
            CUSTOM_X_ZONES, CUSTOM_Y, CUSTOM_Y_FRAC, CUSTOM_Y_ZONES, DISPLAY_UNIT,
            DOCUMENT_BORDER_STYLE, FILE_VERSION_INFO, HOT_SPOT_GRID_ON, HOT_SPOT_GRID_SIZE,
            HOT_SPOT_GRID_SIZE_FRAC, IS_BOC, REFERENCE_ZONE_STYLE, REFERENCE_ZONES_ON,
            SHEET_NUMBER_SPACE_SIZE, SHEET_STYLE, SHOW_BORDER, SHOW_HIDDEN_PINS,
            SHOW_TEMPLATE_GRAPHICS, SNAP_GRID_ON, SNAP_GRID_SIZE, SNAP_GRID_SIZE_FRAC, SYSTEM_FONT,
            TARGET_FILE_NAME, TEMPLATE_FILE_NAME, TITLE_BLOCK_ON, USE_CUSTOM_SHEET, USE_MBCS,
            VISIBLE_GRID_ON, VISIBLE_GRID_SIZE, VISIBLE_GRID_SIZE_FRAC, WORKSPACE_ORIENTATION,
        },
        text::{
            ALIGNMENT, AUTO_SIZE, BOLD, CLIP_TO_RECT, DESCRIPTION, ITALIC, JUSTIFICATION, NAME,
            SHOW_NAME, STRIKE_OUT, TEXT, TEXT_COLOR, TEXT_FONT_ID, TEXT_HORZ_ANCHOR, TEXT_MARGIN,
            TEXT_MARGIN_FRAC, TEXT_STYLE, TEXT_VERT_ANCHOR, UNDERLINE, UNDERLINE_COLOR, WORD_WRAP,
        },
        vault::{
            DATABASE_TABLE_NAME, DESIGN_ITEM_ID, GENERIC_COMPONENT_TEMPLATE_GUID, ITEM_GUID,
            ITEM_REVISION_GUID, LIBRARY_PATH, NOT_ALLOW_DATABASE_SYNCHRONIZE,
            NOT_ALLOW_LIBRARY_SYNCHRONIZE, NOT_USE_DB_TABLE_NAME, PROPS_REVISION_GUID,
            PROPS_VAULT_GUID, RELEASE_ITEM_GUID, RELEASE_VAULT_GUID, REVISION_GUID,
            REVISION_NAME, SOURCE_LIBRARY_NAME, SYMBOL_ITEM_GUID, SYMBOL_REVISION_GUID,
            SYMBOL_VAULT_GUID, TEMPLATE_ITEM_GUID, TEMPLATE_REVISION_GUID,
            TEMPLATE_REVISION_HRID, TEMPLATE_VAULT_GUID, TEMPLATE_VAULT_HRID, VAULT_GUID,
        },
        visual::{
            ARROW_KIND, BORDER_WIDTH, COLOR, CORNER_X, CORNER_X_FRAC, CORNER_X_RADIUS,
            CORNER_X_RADIUS_FRAC, CORNER_Y, CORNER_Y_FRAC, CORNER_Y_RADIUS, CORNER_Y_RADIUS_FRAC,
            EMBED_IMAGE, END_ANGLE, END_LINE_SHAPE, FILE_NAME, FONT_ID, FONT_ID_COUNT, FONT_NAME,
            HEIGHT, IS_SOLID, KEEP_ASPECT, LINE_SHAPE_SIZE, LINE_STYLE, LINE_STYLE_EXT, LINE_WIDTH,
            LOCATION_COUNT, LOCATION_X, LOCATION_X_FRAC, LOCATION_Y, LOCATION_Y_FRAC, MIRROR,
            ORIENTATION, OVERIDE_COLORS, RADIUS, RADIUS_FRAC, ROTATION, SCALE_FACTOR,
            SCALE_FACTOR_FRAC, SECONDARY_RADIUS, SECONDARY_RADIUS_FRAC, SIZE, START_ANGLE,
            START_LINE_SHAPE, STYLE, STYLE_ID, TRANSPARENT, WIDTH, X_SIZE, Y_SIZE,
        },
    },
    sch::{PortArrowStyle, PortIoType, PowerObjectStyle, SchFont},
};

use crate::binary_io::{BinaryReader, BinaryWriter};
use crate::block_stream::{write_binary_block, write_text_block};
use crate::param_collection::ParameterCollection;
use crate::param_value::{SchAngle, ToParamValue};
use crate::{AltiumFormatError, Result};

// ── Base composition types ────────────────────────────────────────────────────

/// Base fields shared by every schematic record: ownership, part/display mode, locking.
/// Field order matches Altium's ExportDataObject + ExportGraphicalObject serialization order.
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchPrimitiveBase {
    #[param(key = OWNER_INDEX, default = 0i32)]
    pub owner_index: i32,
    #[param(key = IS_NOT_ACCESSIBLE, default = false)]
    pub is_not_accessible: bool,
    #[param(key = INDEX_IN_SHEET, default = 0i32)]
    pub index_in_sheet: i32,
    #[param(key = OWNER_PART_ID, default = 0i32)]
    pub owner_part_id: i32,
    #[param(key = OWNER_PART_DISPLAY_MODE, default = 0i32)]
    pub owner_part_display_mode: i32,
    #[param(key = SELECTION_MEMORY, default = 0u8)]
    pub selection_memory: u8,
    #[param(key = GRAPHICALLY_LOCKED, default = false)]
    pub graphically_locked: bool,
    #[param(key = UNION_INDEX, default = 0i32)]
    pub union_index: i32,
    #[param(key = STYLE_ID, default = 0i32)]
    pub style_id: i32,
}

/// Extends `SchPrimitiveBase` with location and color fields for graphical objects.
#[derive(FromParams, ToParams, Debug)]
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
    // Presence flags (SchDoc text pins can omit these keys; preserve omission on write).
    pub symbol_inner_edge_present: bool,
    pub symbol_outer_edge_present: bool,
    pub symbol_inside_present: bool,
    pub symbol_outside_present: bool,
    // SchDoc text-only optional pin fields.
    pub symbol: Option<IeeeSymbol>,
    pub description: String,
    pub formal_type: StdLogicState,
    pub electrical: PinElectricalType,
    pub pin_length: Coord,
    pub location: CoordPoint,
    pub color: Color,
    pub name: String,
    pub designator: String,
    pub swap_id_pin: String,
    pub swap_id_part: String,
    pub swap_id_pair: String,
    pub default_value: String,
    pub spice_pin_name: String,
    pub hidden_net_name: String,
    pub unique_id: String,

    // Decoded from PinConglomerate
    pub orientation: RotationBy90,
    pub is_hidden: bool,
    pub show_name: bool,
    pub show_designator: bool,
    pub is_not_accessible: bool,
    pub graphically_locked: bool,
    pub owner_index_additional_list: bool,

    // Populated by sidecar streams (Milestone 9)
    pub pin_symbol_line_width: Option<i32>,
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
    if binary_code != PIN_BINARY_CODE {
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

    let formal_type = StdLogicState::try_from(r.read_u8()?)?;
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
        symbol_inner_edge_present: true,
        symbol_outer_edge_present: true,
        symbol_inside_present: true,
        symbol_outside_present: true,
        symbol: None,
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
        swap_id_pair: String::new(),
        default_value,
        spice_pin_name: String::new(),
        hidden_net_name: String::new(),
        unique_id: String::new(),
        orientation: cong.orientation,
        is_hidden: cong.is_hidden,
        show_name: cong.show_name,
        show_designator: cong.show_designator,
        is_not_accessible: cong.is_not_accessible,
        graphically_locked: cong.graphically_locked,
        owner_index_additional_list: cong.owner_index_additional_list,
        // Sidecar fields: zero-initialized, populated in M9
        pin_symbol_line_width: None,
        pin_package_length: String::new(),
        propagation_delay: String::new(),
        selected_functions: Vec::new(),
        defined_functions: Vec::new(),
        name_text_data: None,
        designator_text_data: None,
    })
}

/// Parses a text-format schematic pin record (RECORD=2) as used by SchDoc.
pub(crate) fn parse_text_pin(params: &mut ParameterCollection) -> Result<SchPin> {
    let owner_index: i32 = params.remove_with_default(OWNER_INDEX, 0i32)?;
    let owner_part_id: i32 = params.remove_with_default(OWNER_PART_ID, 0i32)?;
    let owner_part_display_mode: u8 =
        params.remove_with_default::<i32>(OWNER_PART_DISPLAY_MODE, 0i32)? as u8;

    let symbol_inner_edge_raw = params.remove_optional::<u8>(SYMBOL_INNER_EDGE)?;
    let symbol_outer_edge_raw = params.remove_optional::<u8>(SYMBOL_OUTER_EDGE)?;
    let symbol_inside_raw = params.remove_optional::<u8>(SYMBOL_INNER)?;
    let symbol_outside_raw = params.remove_optional::<u8>(SYMBOL_OUTER)?;
    let symbol_inner_edge = IeeeSymbol::try_from(symbol_inner_edge_raw.unwrap_or(0u8))?;
    let symbol_outer_edge = IeeeSymbol::try_from(symbol_outer_edge_raw.unwrap_or(0u8))?;
    let symbol_inside = IeeeSymbol::try_from(symbol_inside_raw.unwrap_or(0u8))?;
    let symbol_outside = IeeeSymbol::try_from(symbol_outside_raw.unwrap_or(0u8))?;
    let symbol = params
        .remove_optional::<u8>(SYMBOL)?
        .map(IeeeSymbol::try_from)
        .transpose()?;

    let description: String = params.remove_with_default(DESCRIPTION, String::new())?;
    let formal_type = StdLogicState::try_from(params.remove_with_default::<u8>(FORMAL_TYPE, 0u8)?)?;
    let electrical =
        PinElectricalType::try_from(params.remove_with_default::<u8>(ELECTRICAL, 0u8)?)?;

    let pin_conglomerate = params.remove_with_default(PIN_CONGLOMERATE, 0u8)?;
    let cong = decode_pin_conglomerate(pin_conglomerate)?;

    let pin_length = params
        .remove_coord_optional(PIN_LENGTH, "PinLength_Frac")?
        .unwrap_or_else(|| Coord::from_internal(0));
    let location = CoordPoint::new(
        params
            .remove_coord_optional(LOCATION_X, LOCATION_X_FRAC)?
            .unwrap_or_else(|| Coord::from_internal(0)),
        params
            .remove_coord_optional(LOCATION_Y, LOCATION_Y_FRAC)?
            .unwrap_or_else(|| Coord::from_internal(0)),
    );

    let color = params
        .remove_optional::<i32>(COLOR)?
        .map(Color::new)
        .unwrap_or(Color::BLACK);

    let name: String = params.remove_with_default(NAME, String::new())?;
    let designator: String = params.remove_with_default("Designator", String::new())?;
    let swap_id_pin: String = params.remove_with_default(SWAP_ID_PIN, String::new())?;
    let swap_id_part: String = params.remove_with_default(SWAP_ID_PART, String::new())?;
    let default_value: String = params.remove_with_default(DEF_VALUE, String::new())?;

    // Optional SchDoc pin fields.
    let pin_symbol_line_width = params.remove_optional::<i32>(SYMBOL_LINE_WIDTH)?;
    let spice_pin_name = params
        .remove_optional::<String>("SpicePinName")?
        .unwrap_or_default();
    let hidden_net_name = params
        .remove_optional::<String>("HiddenNetName")?
        .unwrap_or_default();
    let unique_id = params
        .remove_optional::<String>(UNIQUE_ID)?
        .unwrap_or_default();
    let swap_id_pair = params
        .remove_optional::<String>(SWAP_ID_PAIR)?
        .unwrap_or_default();

    let pin_package_length = params
        .remove_optional::<String>(PIN_PACKAGE_LENGTH_KEY)?
        .unwrap_or_default();
    let propagation_delay = params
        .remove_optional::<String>(PIN_PROPAGATION_DELAY_KEY)?
        .unwrap_or_default();

    // Optional text positioning/font overrides in SchDoc ASCII pin records.
    let name_text_data =
        if let Some(cong) = params.remove_optional::<u8>(PIN_NAME_POSITION_CONGLOMERATE)? {
            let position_mode_custom = (cong & 0x01) != 0;
            let rotation_anchor_component = (cong & 0x02) != 0;
            let rotation_relative = RotationBy90::try_from((cong & 0x0C) >> 2)?;
            let font_mode_custom = (cong & 0x10) != 0;

            let custom_position_margin = if position_mode_custom {
                params.remove_coord_optional(
                    NAME_CUSTOM_POSITION_MARGIN,
                    &format!("{NAME_CUSTOM_POSITION_MARGIN}_Frac"),
                )?
            } else {
                None
            };

            let custom_font_id = if font_mode_custom {
                params.remove_optional::<i16>(NAME_CUSTOM_FONT_ID)?
            } else {
                None
            };

            let custom_color = if font_mode_custom {
                params.remove_optional::<Color>(NAME_CUSTOM_COLOR)?
            } else {
                None
            };

            Some(PinTextPositioning {
                position_mode_custom,
                rotation_anchor_component,
                rotation_relative,
                font_mode_custom,
                custom_position_margin,
                custom_font_id,
                custom_color,
            })
        } else {
            None
        };

    let designator_text_data =
        if let Some(cong) = params.remove_optional::<u8>(PIN_DESIGNATOR_POSITION_CONGLOMERATE)? {
            let position_mode_custom = (cong & 0x01) != 0;
            let rotation_anchor_component = (cong & 0x02) != 0;
            let rotation_relative = RotationBy90::try_from((cong & 0x0C) >> 2)?;
            let font_mode_custom = (cong & 0x10) != 0;

            let custom_position_margin = if position_mode_custom {
                params.remove_coord_optional(
                    DESIGNATOR_CUSTOM_POSITION_MARGIN,
                    &format!("{DESIGNATOR_CUSTOM_POSITION_MARGIN}_Frac"),
                )?
            } else {
                None
            };

            let custom_font_id = if font_mode_custom {
                params.remove_optional::<i16>(DESIGNATOR_CUSTOM_FONT_ID)?
            } else {
                None
            };

            let custom_color = if font_mode_custom {
                params.remove_optional::<Color>(DESIGNATOR_CUSTOM_COLOR)?
            } else {
                None
            };

            Some(PinTextPositioning {
                position_mode_custom,
                rotation_anchor_component,
                rotation_relative,
                font_mode_custom,
                custom_position_margin,
                custom_font_id,
                custom_color,
            })
        } else {
            None
        };

    let selected_functions_count: i32 =
        params.remove_with_default(PIN_SELECTED_FUNCTIONS_COUNT, 0i32)?;
    let mut selected_functions = Vec::with_capacity(selected_functions_count as usize);
    for i in 1..=selected_functions_count {
        let key = format!("{PIN_SELECTED_FUNCTION}{i}");
        selected_functions.push(params.remove_with_default::<String>(&key, String::new())?);
    }

    let defined_functions_count: i32 =
        params.remove_with_default(PIN_DEFINED_FUNCTIONS_COUNT, 0i32)?;
    let mut defined_functions = Vec::with_capacity(defined_functions_count as usize);
    for i in 1..=defined_functions_count {
        let key = format!("{PIN_DEFINED_FUNCTION}{i}");
        defined_functions.push(params.remove_with_default::<String>(&key, String::new())?);
    }

    Ok(SchPin {
        owner_index,
        owner_part_id,
        owner_part_display_mode,
        symbol_inner_edge,
        symbol_outer_edge,
        symbol_inside,
        symbol_outside,
        symbol_inner_edge_present: symbol_inner_edge_raw.is_some(),
        symbol_outer_edge_present: symbol_outer_edge_raw.is_some(),
        symbol_inside_present: symbol_inside_raw.is_some(),
        symbol_outside_present: symbol_outside_raw.is_some(),
        symbol,
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
        swap_id_pair,
        default_value,
        spice_pin_name,
        hidden_net_name,
        unique_id,
        orientation: cong.orientation,
        is_hidden: cong.is_hidden,
        show_name: cong.show_name,
        show_designator: cong.show_designator,
        is_not_accessible: cong.is_not_accessible,
        graphically_locked: cong.graphically_locked,
        owner_index_additional_list: cong.owner_index_additional_list,
        pin_symbol_line_width,
        pin_package_length,
        propagation_delay,
        selected_functions,
        defined_functions,
        name_text_data,
        designator_text_data,
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
    pub index_in_sheet: i32,
    pub owner_part_id: i32,
    pub owner_part_display_mode: i32,
    pub graphically_locked: bool,
    pub union_index: i32,
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

pub(crate) fn parse_component_record(
    params: &mut crate::param_collection::ParameterCollection,
) -> crate::Result<SchComponent> {
    let lib_reference: String = params.remove_with_default(LIB_REFERENCE, String::new())?;
    let component_description: String =
        params.remove_with_default(COMPONENT_DESCRIPTION, String::new())?;
    let part_count: i32 = params.remove_with_default(PART_COUNT, 1i32)?;
    let display_mode_count: i32 = params.remove_with_default(DISPLAY_MODE_COUNT, 0i32)?;

    // ExportDataObject + ExportGraphicalObject fields (order matches Altium export)
    let owner_index: i32 = params.remove_with_default(OWNER_INDEX, 0i32)?;
    let is_not_accessible: bool = params.remove_with_default(IS_NOT_ACCESSIBLE, false)?;
    let index_in_sheet: i32 = params.remove_with_default(INDEX_IN_SHEET, 0i32)?;
    let owner_part_id: i32 = params.remove_with_default(OWNER_PART_ID, 0i32)?;
    let owner_part_display_mode: i32 = params.remove_with_default(OWNER_PART_DISPLAY_MODE, 0i32)?;
    let graphically_locked: bool = params.remove_with_default(GRAPHICALLY_LOCKED, false)?;
    let union_index: i32 = params.remove_with_default(UNION_INDEX, 0i32)?;

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
    let orientation: RotationBy90 =
        params.remove_with_default(ORIENTATION, RotationBy90::Rotate0)?;
    let current_part_id: i32 = params.remove_with_default(CURRENT_PART_ID, 1i32)?;
    let show_hidden_fields: bool = params.remove_with_default(SHOW_HIDDEN_FIELDS, false)?;
    let show_hidden_pins: bool = params.remove_with_default(SHOW_HIDDEN_PINS, false)?;
    let library_path: String = params.remove_with_default(LIBRARY_PATH, String::new())?;
    let source_library_name: String =
        params.remove_with_default(SOURCE_LIBRARY_NAME, String::new())?;
    let database_table_name: String =
        params.remove_with_default(DATABASE_TABLE_NAME, String::new())?;
    let sheet_part_file_name: String =
        params.remove_with_default(SHEET_PART_FILE_NAME, String::new())?;
    let target_file_name: String = params.remove_with_default(TARGET_FILE_NAME, String::new())?;
    let unique_id: String = params.remove_with_default(UNIQUE_ID, String::new())?;
    let area_color: Color = params.remove_with_default(AREA_COLOR, Color::BLACK)?;
    let color: Color = params.remove_with_default(COLOR, Color::BLACK)?;
    let pin_color: Color = params.remove_with_default(PIN_COLOR, Color::BLACK)?;
    let override_colors: bool = params.remove_with_default(OVERIDE_COLORS, false)?;
    let display_field_names: bool = params.remove_with_default(DISPLAY_FIELD_NAMES, false)?;
    let designator_locked: bool = params.remove_with_default(DESIGNATOR_LOCKED, false)?;
    // C# Import_Boolean_WithDefault: defaults to DesignatorLocked value when absent
    let part_id_locked: bool = params.remove_with_default(PART_ID_LOCKED, designator_locked)?;
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
    let symbol_revision_guid: String =
        params.remove_with_default(SYMBOL_REVISION_GUID, String::new())?;
    let generic_component_template_guid: String =
        params.remove_with_default(GENERIC_COMPONENT_TEMPLATE_GUID, String::new())?;
    let has_only_current_part_info: bool =
        params.remove_with_default(HAS_ONLY_CURRENT_PART_INFO, false)?;
    let all_pin_count: i32 = params.remove_with_default(ALL_PIN_COUNT, 0i32)?;
    let key_component_unique_id: String =
        params.remove_with_default(KEY_COMPONENT_UNIQUE_ID, String::new())?;
    let component_kind: ComponentKind =
        params.remove_with_default(COMPONENT_KIND, ComponentKind::Standard)?;
    let component_kind_version2: ComponentKind =
        params.remove_with_default(COMPONENT_KIND_VERSION2, ComponentKind::Standard)?;
    let component_kind_version3: ComponentKind =
        params.remove_with_default(COMPONENT_KIND_VERSION3, ComponentKind::Standard)?;

    // CustomDisplayModeName0..N-1
    let mut custom_display_mode_names = Vec::with_capacity(display_mode_count as usize);
    for i in 0..display_mode_count {
        let key = format!(
            "{}{}",
            altium_format_types::constants::component::CUSTOM_DISPLAY_MODE_NAME,
            i
        );
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
        index_in_sheet,
        owner_part_id,
        owner_part_display_mode,
        graphically_locked,
        union_index,
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
///
/// Field order matches Altium's `ExportLine` (FileFormatV5.cs:1805-1821):
/// ExportGraphicalObject, Location, Corner, LineWidth, LineStyle, Color, LineStyleExt, UniqueID
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchLine {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,
    #[param(coord_point, x_key = CORNER_X, x_frac = CORNER_X_FRAC, y_key = CORNER_Y, y_frac = CORNER_Y_FRAC)]
    pub corner: CoordPoint,
    #[param(key = LINE_WIDTH, default = PenWidth::Zero)]
    pub line_width: PenWidth,
    #[param(key = LINE_STYLE, default = LineStyle::Solid)]
    pub line_style: LineStyle,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = LINE_STYLE_EXT, default = LineStyle::Solid)]
    pub line_style_ext: LineStyle,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}

/// A filled or outlined rectangle (RECORD=14).
///
/// Field order matches Altium's `ExportRectangle` (FileFormatV5.cs:1620-1637):
/// ExportGraphicalObject, Location, Corner, LineStyleExt, LineWidth, Color, AreaColor,
/// IsSolid, Transparent, UniqueID
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchRectangle {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,
    #[param(coord_point, x_key = CORNER_X, x_frac = CORNER_X_FRAC, y_key = CORNER_Y, y_frac = CORNER_Y_FRAC)]
    pub corner: CoordPoint,
    #[param(key = LINE_STYLE_EXT, default = LineStyle::Solid)]
    pub line_style: LineStyle,
    #[param(key = LINE_WIDTH, default = PenWidth::Zero)]
    pub line_width: PenWidth,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = AREA_COLOR, default = Color::BLACK)]
    pub area_color: Color,
    #[param(key = IS_SOLID, default = false)]
    pub is_solid: bool,
    #[param(key = TRANSPARENT, default = false)]
    pub transparent: bool,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}

/// A rounded rectangle (RECORD=10).
///
/// Field order matches Altium's `ExportRoundRectangle` (FileFormatV5.cs:1680-1697):
/// ExportGraphicalObject, Location, Corner, CornerXRadius, CornerYRadius, LineWidth,
/// Color, AreaColor, IsSolid, UniqueID
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchRoundRectangle {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,
    #[param(coord_point, x_key = CORNER_X, x_frac = CORNER_X_FRAC, y_key = CORNER_Y, y_frac = CORNER_Y_FRAC)]
    pub corner: CoordPoint,
    #[param(coord, key = CORNER_X_RADIUS, frac_key = CORNER_X_RADIUS_FRAC)]
    pub corner_x_radius: Coord,
    #[param(coord, key = CORNER_Y_RADIUS, frac_key = CORNER_Y_RADIUS_FRAC)]
    pub corner_y_radius: Coord,
    #[param(key = LINE_WIDTH, default = PenWidth::Zero)]
    pub line_width: PenWidth,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = AREA_COLOR, default = Color::BLACK)]
    pub area_color: Color,
    #[param(key = IS_SOLID, default = false)]
    pub is_solid: bool,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}

/// A circular arc segment (RECORD=12).
///
/// Field order matches Altium's `ExportArc` (FileFormatV5.cs:177-191):
/// ExportGraphicalObject, Location, Radius, LineWidth, StartAngle, EndAngle, Color, UniqueID
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchArc {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,
    #[param(coord, key = RADIUS, frac_key = RADIUS_FRAC)]
    pub radius: Coord,
    #[param(key = LINE_WIDTH, default = PenWidth::Zero)]
    pub line_width: PenWidth,
    #[param(key = START_ANGLE, default = SchAngle(0.0))]
    pub start_angle: SchAngle,
    #[param(key = END_ANGLE, optional)]
    pub end_angle: Option<SchAngle>,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}

/// An elliptical arc segment (RECORD=11).
///
/// Field order matches Altium's `ExportEllipticalArc` (FileFormatV5.cs:225-240):
/// ExportGraphicalObject, Location, Radius, SecondaryRadius, LineWidth, StartAngle,
/// EndAngle, Color, UniqueID
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchEllipticalArc {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,
    #[param(coord, key = RADIUS, frac_key = RADIUS_FRAC)]
    pub radius: Coord,
    #[param(coord, key = SECONDARY_RADIUS, frac_key = SECONDARY_RADIUS_FRAC)]
    pub secondary_radius: Coord,
    #[param(key = LINE_WIDTH, default = PenWidth::Zero)]
    pub line_width: PenWidth,
    #[param(key = START_ANGLE, default = SchAngle(0.0))]
    pub start_angle: SchAngle,
    #[param(key = END_ANGLE, optional)]
    pub end_angle: Option<SchAngle>,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}

/// A filled or outlined ellipse (RECORD=8).
///
/// Field order matches Altium's `ExportEllipse` (FileFormatV5.cs:329-345):
/// ExportGraphicalObject, Location, Radius, SecondaryRadius, LineWidth, Color, AreaColor,
/// IsSolid, Transparent, UniqueID
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchEllipse {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,
    #[param(coord, key = RADIUS, frac_key = RADIUS_FRAC)]
    pub radius: Coord,
    #[param(coord, key = SECONDARY_RADIUS, frac_key = SECONDARY_RADIUS_FRAC)]
    pub secondary_radius: Coord,
    #[param(key = LINE_WIDTH, default = PenWidth::Zero)]
    pub line_width: PenWidth,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = AREA_COLOR, default = Color::BLACK)]
    pub area_color: Color,
    #[param(key = IS_SOLID, default = false)]
    pub is_solid: bool,
    #[param(key = TRANSPARENT, default = false)]
    pub transparent: bool,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}

/// A pie (filled wedge) shape (RECORD=9).
///
/// Field order matches Altium's `ExportPie` (FileFormatV5.cs:277-292):
/// ExportGraphicalObject, Location, Radius, LineWidth, StartAngle, EndAngle,
/// Color, AreaColor, IsSolid
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchPie {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,
    #[param(coord, key = RADIUS, frac_key = RADIUS_FRAC)]
    pub radius: Coord,
    #[param(key = LINE_WIDTH, default = PenWidth::Zero)]
    pub line_width: PenWidth,
    #[param(key = START_ANGLE, default = SchAngle(0.0))]
    pub start_angle: SchAngle,
    #[param(key = END_ANGLE, optional)]
    pub end_angle: Option<SchAngle>,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = AREA_COLOR, default = Color::BLACK)]
    pub area_color: Color,
    #[param(key = IS_SOLID, default = false)]
    pub is_solid: bool,
}

/// A multi-segment polyline (RECORD=6).
///
/// Field order matches Altium's `ExportPolyline` (FileFormatV5.cs:1175-1191):
/// ExportGraphicalObject, LineWidth, LineStyle, StartLineShape, EndLineShape,
/// LineShapeSize, Color, Vertices, LineStyleExt, UniqueID
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchPolyline {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(key = LINE_WIDTH, default = PenWidth::Zero)]
    pub line_width: PenWidth,
    #[param(key = LINE_STYLE, default = LineStyle::Solid)]
    pub line_style: LineStyle,
    #[param(key = START_LINE_SHAPE, default = LineShape::None)]
    pub start_line_shape: LineShape,
    #[param(key = END_LINE_SHAPE, default = LineShape::None)]
    pub end_line_shape: LineShape,
    #[param(key = LINE_SHAPE_SIZE, default = PenWidth::Zero)]
    pub line_shape_size: PenWidth,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(indexed_coords, count_key = LOCATION_COUNT, x_prefix = "X", y_prefix = "Y")]
    pub vertices: Vec<CoordPoint>,
    #[param(key = LINE_STYLE_EXT, default = LineStyle::Solid)]
    pub line_style_ext: LineStyle,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}

/// A closed polygon (RECORD=7).
///
/// Field order matches Altium's `ExportPolygon` (FileFormatV5.cs:1133-1146):
/// ExportGraphicalObject, LineWidth, Color, AreaColor, IsSolid, Transparent, Vertices, UniqueID
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchPolygon {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(key = LINE_WIDTH, default = PenWidth::Zero)]
    pub line_width: PenWidth,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = AREA_COLOR, default = Color::BLACK)]
    pub area_color: Color,
    #[param(key = IS_SOLID, default = false)]
    pub is_solid: bool,
    #[param(key = TRANSPARENT, default = false)]
    pub transparent: bool,
    #[param(indexed_coords, count_key = LOCATION_COUNT, x_prefix = "X", y_prefix = "Y")]
    pub vertices: Vec<CoordPoint>,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}

/// A Bezier curve (RECORD=5).
///
/// Field order matches Altium's `ExportBezier` (FileFormatV5.cs:1225-1235):
/// ExportGraphicalObject, LineWidth, Color, Vertices, UniqueID
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchBezier {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(key = LINE_WIDTH, default = PenWidth::Zero)]
    pub line_width: PenWidth,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(indexed_coords, count_key = LOCATION_COUNT, x_prefix = "X", y_prefix = "Y")]
    pub vertices: Vec<CoordPoint>,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}

/// An embedded or linked image (RECORD=30).
///
/// Field order matches Altium's `ExportImage` (FileFormatV5.cs:1740-1758):
/// ExportGraphicalObject, Location, Corner, Orientation, LineWidth, Color,
/// IsSolid, KeepAspect, EmbedImage, FileName, UniqueID
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchImage {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,
    #[param(coord_point, x_key = CORNER_X, x_frac = CORNER_X_FRAC, y_key = CORNER_Y, y_frac = CORNER_Y_FRAC)]
    pub corner: CoordPoint,
    #[param(key = ORIENTATION, default = RotationBy90::Rotate0)]
    pub orientation: RotationBy90,
    #[param(key = LINE_WIDTH, default = PenWidth::Zero)]
    pub line_width: PenWidth,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = IS_SOLID, default = false)]
    pub is_solid: bool,
    #[param(key = KEEP_ASPECT, default = false)]
    pub keep_aspect: bool,
    #[param(key = EMBED_IMAGE, default = false)]
    pub embed_image: bool,
    #[param(key = FILE_NAME, default = String::new())]
    pub file_name: String,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}

// ── Text and annotation records ───────────────────────────────────────────────

/// A text label (RECORD=4).
///
/// Field order matches Altium's `ExportLabel` (FileFormatV5.cs:868-886).
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchLabel {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,
    #[param(key = ORIENTATION, default = RotationBy90::Rotate0)]
    pub orientation: RotationBy90,
    #[param(key = JUSTIFICATION, default = TextJustification::BottomLeft)]
    pub justification: TextJustification,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = FONT_ID, default = 1i32)]
    pub font_id: i32,
    #[param(key = TEXT, default = String::new())]
    pub text: String,
    #[param(key = IS_MIRRORED, default = false)]
    pub is_mirrored: bool,
    #[param(key = URL, default = String::new())]
    pub url: String,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}

/// An IEEE symbol shape (RECORD=3).
///
/// Field order matches Altium's `ExportSymbol` (FileFormatV5.cs):
/// ExportGraphicalObject, Symbol, Location, ScaleFactor, Orientation, LineWidth, Color, Mirror
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchSymbol {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(key = SYMBOL, default = IeeeSymbol::NoSymbol)]
    pub symbol: IeeeSymbol,
    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,
    #[param(coord, key = SCALE_FACTOR, frac_key = SCALE_FACTOR_FRAC)]
    pub scale_factor: Coord,
    #[param(key = ORIENTATION, default = RotationBy90::Rotate0)]
    pub orientation: RotationBy90,
    #[param(key = LINE_WIDTH, default = PenWidth::Zero)]
    pub line_width: PenWidth,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = MIRROR, default = false)]
    pub is_mirrored: bool,
}

/// A designator annotation (RECORD=34).
///
/// Field order matches Altium's `ExportParameter` (FileFormatV5.cs:1339-1371).
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchDesignator {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,
    #[param(key = ORIENTATION, default = RotationBy90::Rotate0)]
    pub orientation: RotationBy90,
    #[param(key = JUSTIFICATION, default = TextJustification::BottomLeft)]
    pub justification: TextJustification,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = FONT_ID, default = 1i32)]
    pub font_id: i32,
    #[param(key = IS_HIDDEN, default = false)]
    pub is_hidden: bool,
    #[param(tier2, key = TEXT, default = String::from("*"))]
    pub text: String,
    #[param(tier2, key = NAME, default = String::from("Designator"))]
    pub name: String,
    #[param(key = SHOW_NAME, default = false)]
    pub show_name: bool,
    #[param(key = READ_ONLY_STATE, default = ParameterReadOnlyState::Name)]
    pub read_only_state: ParameterReadOnlyState,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
    #[param(key = NOT_AUTO_POSITION, default = false)]
    pub not_auto_position: bool,
    #[param(key = OVERRIDE_NOT_AUTO_POSITION, default = false)]
    pub override_not_auto_position: bool,
    #[param(key = IS_MIRRORED, default = false)]
    pub is_mirrored: bool,
    #[param(key = NOT_ALLOW_LIBRARY_SYNCHRONIZE, default = false)]
    pub not_allow_library_synchronize: bool,
    #[param(key = NOT_ALLOW_DATABASE_SYNCHRONIZE, default = false)]
    pub not_allow_database_synchronize: bool,
    #[param(key = DESCRIPTION, default = String::new())]
    pub description: String,
    #[param(key = PARAM_TYPE, default = ParameterType::String)]
    pub param_type: ParameterType,
    #[param(key = TEXT_HORZ_ANCHOR, default = TextHorzAnchor::None)]
    pub text_horz_anchor: TextHorzAnchor,
    #[param(key = TEXT_VERT_ANCHOR, default = TextVertAnchor::None)]
    pub text_vert_anchor: TextVertAnchor,
    #[param(key = IS_IMAGE_PARAMETER, default = false)]
    pub is_image_parameter: bool,
}

/// A parameter annotation (RECORD=41).
///
/// Used for Comment, Value, and user-defined parameters. The `name` field
/// determines the parameter's role (e.g., `"Comment"`, `"Value"`).
///
/// Field order matches Altium's `ExportParameter` (FileFormatV5.cs:1339-1371).
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchParameter {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,
    #[param(key = ORIENTATION, default = RotationBy90::Rotate0)]
    pub orientation: RotationBy90,
    #[param(key = JUSTIFICATION, default = TextJustification::BottomLeft)]
    pub justification: TextJustification,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = FONT_ID, default = 1i32)]
    pub font_id: i32,
    #[param(key = IS_HIDDEN, default = false)]
    pub is_hidden: bool,
    #[param(key = TEXT, default = String::new())]
    pub text: String,
    #[param(key = PARAM_TYPE, default = ParameterType::String)]
    pub param_type: ParameterType,
    #[param(tier2, key = NAME, default = String::from("Comment"))]
    pub name: String,
    #[param(key = SHOW_NAME, default = false)]
    pub show_name: bool,
    #[param(key = READ_ONLY_STATE, default = ParameterReadOnlyState::None)]
    pub read_only_state: ParameterReadOnlyState,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
    #[param(key = DESCRIPTION, default = String::new())]
    pub description: String,
    #[param(key = NOT_ALLOW_LIBRARY_SYNCHRONIZE, default = false)]
    pub not_allow_library_synchronize: bool,
    #[param(key = NOT_ALLOW_DATABASE_SYNCHRONIZE, default = false)]
    pub not_allow_database_synchronize: bool,
    #[param(key = NOT_AUTO_POSITION, default = false)]
    pub not_auto_position: bool,
    #[param(key = OVERRIDE_NOT_AUTO_POSITION, default = false)]
    pub override_not_auto_position: bool,
    #[param(key = IS_MIRRORED, default = false)]
    pub is_mirrored: bool,
    #[param(key = TEXT_HORZ_ANCHOR, default = TextHorzAnchor::None)]
    pub text_horz_anchor: TextHorzAnchor,
    #[param(key = TEXT_VERT_ANCHOR, default = TextVertAnchor::None)]
    pub text_vert_anchor: TextVertAnchor,
    #[param(key = IS_IMAGE_PARAMETER, default = false)]
    pub is_image_parameter: bool,
}

/// A text frame (bordered text box) (RECORD=28).
///
/// Field order matches Altium's `ExportTextFrame` (FileFormatV5.cs:1908-1931):
/// ExportGraphicalObject, Location, Corner, LineWidth, Color, AreaColor, TextColor,
/// FontID, IsSolid, ShowBorder, Alignment, WordWrap, ClipToRect, Text, TextMargin,
/// Transparent, UniqueID
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchTextFrame {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,
    #[param(coord_point, x_key = CORNER_X, x_frac = CORNER_X_FRAC, y_key = CORNER_Y, y_frac = CORNER_Y_FRAC)]
    pub corner: CoordPoint,
    #[param(key = LINE_WIDTH, default = PenWidth::Zero)]
    pub line_width: PenWidth,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = AREA_COLOR, default = Color::BLACK)]
    pub area_color: Color,
    #[param(key = TEXT_COLOR, default = Color::BLACK)]
    pub text_color: Color,
    #[param(key = FONT_ID, default = 1i32)]
    pub font_id: i32,
    #[param(key = IS_SOLID, default = false)]
    pub is_solid: bool,
    #[param(key = SHOW_BORDER, default = false)]
    pub show_border: bool,
    #[param(key = ALIGNMENT, default = HorizontalAlign::Left)]
    pub alignment: HorizontalAlign,
    #[param(key = WORD_WRAP, default = false)]
    pub word_wrap: bool,
    #[param(key = CLIP_TO_RECT, default = false)]
    pub clip_to_rect: bool,
    #[param(key = TEXT, default = String::new())]
    pub text: String,
    #[param(coord, key = TEXT_MARGIN, frac_key = TEXT_MARGIN_FRAC)]
    pub text_margin: Coord,
    #[param(key = TRANSPARENT, default = false)]
    pub transparent: bool,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}

// ── Implementation/model records ─────────────────────────────────────────────

/// Container for component implementation (footprint) entries (RECORD=44).
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchImplementationList {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
}

/// A single datafile link within a SchImplementation.
#[derive(Debug, Clone)]
pub(crate) struct ModelDatafileLink {
    pub location: String,
    pub entity_name: String,
    pub file_kind: String,
}

/// A single footprint/model assignment (RECORD=45).
///
/// Field order matches Altium's `ExportImplementation` (FileFormatV5.cs:2510-2540):
/// ExportDataObject, Description, UseComponentLibrary, ModelName, ModelType,
/// DatafileCount, ModelVaultGUID, ModelItemGUID, ModelRevisionGUID,
/// ModelDatafile{i}, ModelDatafileEntity{i}, ModelDatafileKind{i},
/// IsCurrent, DatalinksLocked, DatabaseDatalinksLocked, IntegratedModel,
/// DatabaseModel, UniqueID
#[derive(Debug)]
pub(crate) struct SchImplementation {
    pub base: SchPrimitiveBase,
    pub description: String,
    pub use_component_library: bool,
    pub model_name: String,
    pub model_type: String,
    pub model_vault_guid: String,
    pub model_item_guid: String,
    pub model_revision_guid: String,
    pub datafile_links: Vec<ModelDatafileLink>,
    pub is_current: bool,
    pub datalinks_locked: bool,
    pub database_datalinks_locked: bool,
    pub integrated_model: bool,
    pub database_model: bool,
    pub unique_id: String,
    pub model_location: String,
}

impl SchImplementation {
    pub fn from_params(params: &mut ParameterCollection) -> crate::Result<Self> {
        let base = SchPrimitiveBase::from_params(params)?;
        let description: String = params.remove_with_default(DESCRIPTION, String::new())?;
        let use_component_library: bool = params.remove_with_default(USE_COMPONENT_LIBRARY, false)?;
        let model_name: String = params.remove_with_default(MODEL_NAME, String::new())?;
        let model_type: String = params.remove_with_default(MODEL_TYPE, String::new())?;
        let datafile_count: i32 = params.remove_with_default(DATAFILE_COUNT, 0i32)?;
        let model_vault_guid: String = params.remove_with_default(MODEL_VAULT_GUID, String::new())?;
        let model_item_guid: String = params.remove_with_default(MODEL_ITEM_GUID, String::new())?;
        let model_revision_guid: String = params.remove_with_default(MODEL_REVISION_GUID, String::new())?;

        let mut datafile_links = Vec::with_capacity(datafile_count.max(0) as usize);
        for i in 0..datafile_count.max(0) {
            let location: String = params.remove_with_default(&format!("ModelDatafile{i}"), String::new())?;
            let entity_name: String = params.remove_with_default(&format!("ModelDatafileEntity{i}"), String::new())?;
            let file_kind: String = params.remove_with_default(&format!("ModelDatafileKind{i}"), String::new())?;
            datafile_links.push(ModelDatafileLink { location, entity_name, file_kind });
        }
        // Consume any extra entries beyond datafile_count (older files may have them).
        let mut extra_i = datafile_count.max(0) as usize;
        loop {
            let key = format!("ModelDatafile{extra_i}");
            match params.remove_optional::<String>(&key)? {
                Some(location) => {
                    let entity_name: String = params.remove_with_default(&format!("ModelDatafileEntity{extra_i}"), String::new())?;
                    let file_kind: String = params.remove_with_default(&format!("ModelDatafileKind{extra_i}"), String::new())?;
                    datafile_links.push(ModelDatafileLink { location, entity_name, file_kind });
                }
                None => break,
            }
            extra_i += 1;
        }

        let is_current: bool = params.remove_with_default(IS_CURRENT, false)?;
        let datalinks_locked: bool = params.remove_with_default(DATALINKS_LOCKED, false)?;
        let database_datalinks_locked: bool = params.remove_with_default(DATABASE_DATALINKS_LOCKED, false)?;
        let integrated_model: bool = params.remove_with_default(INTEGRATED_MODEL, false)?;
        let database_model: bool = params.remove_with_default(DATABASE_MODEL, false)?;
        let unique_id: String = params.remove_with_default(UNIQUE_ID, String::new())?;
        let model_location: String = params.remove_with_default(MODEL_LOCATION, String::new())?;

        Ok(SchImplementation {
            base,
            description,
            use_component_library,
            model_name,
            model_type,
            model_vault_guid,
            model_item_guid,
            model_revision_guid,
            datafile_links,
            is_current,
            datalinks_locked,
            database_datalinks_locked,
            integrated_model,
            database_model,
            unique_id,
            model_location,
        })
    }
}

/// Container for pin-to-pad mapping entries (RECORD=46).
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchImplementationMap {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
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
        Ok(SchMapDefiner {
            base,
            des_intf,
            des_imps,
        })
    }
}

/// A parameter list container (RECORD=48).
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchParameterList {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
}

/// A schematic sheet record (RECORD=31).
///
/// Carries the sheet font table and document display settings.
#[derive(Debug)]
pub(crate) struct SchSheet {
    pub base: SchPrimitiveBase,
    pub fonts: Vec<SchFont>,
    pub display_settings: SchDisplaySettings,
    pub template_vault_guid: String,
    pub template_item_guid: String,
    pub template_revision_guid: String,
    pub template_vault_hrid: String,
    pub template_revision_hrid: String,
    pub release_vault_guid: String,
    pub release_item_guid: String,
    pub item_revision_guid: String,
    pub props_vault_guid: String,
    pub props_revision_guid: String,
}

impl SchSheet {
    pub(crate) fn from_params(params: &mut ParameterCollection) -> Result<Self> {
        let base = SchPrimitiveBase::from_params(params)?;

        let fonts = params.remove_indexed(FONT_ID_COUNT, 1, |p, i| {
            let idx = i.to_string();
            let name: String = p.remove_required(&format!("{FONT_NAME}{idx}"))?;
            let size: i32 = p.remove_required(&format!("{SIZE}{idx}"))?;
            let rotation: i32 = p.remove_with_default(&format!("{ROTATION}{idx}"), 0i32)?;
            let bold: bool = p.remove_with_default(&format!("{BOLD}{idx}"), false)?;
            let italic: bool = p.remove_with_default(&format!("{ITALIC}{idx}"), false)?;
            let underline: bool = p.remove_with_default(&format!("{UNDERLINE}{idx}"), false)?;
            let strikeout: bool = p.remove_with_default(&format!("{STRIKE_OUT}{idx}"), false)?;
            Ok(SchFont {
                id: i as i32,
                name,
                size,
                rotation,
                bold,
                italic,
                underline,
                strikeout,
            })
        })?;

        let display_settings = SchDisplaySettings {
            snap_grid_on: params.remove_optional(SNAP_GRID_ON)?,
            snap_grid_size: params.remove_coord_optional(SNAP_GRID_SIZE, SNAP_GRID_SIZE_FRAC)?,
            visible_grid_on: params.remove_optional(VISIBLE_GRID_ON)?,
            visible_grid_size: params
                .remove_coord_optional(VISIBLE_GRID_SIZE, VISIBLE_GRID_SIZE_FRAC)?,
            hot_spot_grid_on: params.remove_optional(HOT_SPOT_GRID_ON)?,
            hot_spot_grid_size: params
                .remove_coord_optional(HOT_SPOT_GRID_SIZE, HOT_SPOT_GRID_SIZE_FRAC)?,
            sheet_style: params
                .remove_optional::<u8>(SHEET_STYLE)?
                .map(SheetStyle::try_from)
                .transpose()?,
            use_custom_sheet: params.remove_optional(USE_CUSTOM_SHEET)?,
            custom_x: params.remove_coord_optional(CUSTOM_X, CUSTOM_X_FRAC)?,
            custom_y: params.remove_coord_optional(CUSTOM_Y, CUSTOM_Y_FRAC)?,
            border_on: params.remove_optional(BORDER_ON)?,
            title_block_on: params.remove_optional(TITLE_BLOCK_ON)?,
            document_border_style: params
                .remove_optional::<u8>(DOCUMENT_BORDER_STYLE)?
                .map(SheetBorderStyle::try_from)
                .transpose()?,
            reference_zones_on: params.remove_optional(REFERENCE_ZONES_ON)?,
            reference_zone_style: params
                .remove_optional::<u8>(REFERENCE_ZONE_STYLE)?
                .map(SheetReferenceZoneStyle::try_from)
                .transpose()?,
            custom_x_zones: params.remove_optional(CUSTOM_X_ZONES)?,
            custom_y_zones: params.remove_optional(CUSTOM_Y_ZONES)?,
            custom_margin_width: params.remove_coord_optional(
                CUSTOM_MARGIN_WIDTH,
                &format!("{CUSTOM_MARGIN_WIDTH}_Frac"),
            )?,
            sheet_number_space_size: params.remove_optional(SHEET_NUMBER_SPACE_SIZE)?,
            workspace_orientation: params
                .remove_optional::<u8>(WORKSPACE_ORIENTATION)?
                .map(SheetOrientation::try_from)
                .transpose()?,
            show_hidden_pins: params.remove_optional(SHOW_HIDDEN_PINS)?,
            show_template_graphics: params.remove_optional(SHOW_TEMPLATE_GRAPHICS)?,
            always_show_cd: None,
            template_file_name: params.remove_optional(TEMPLATE_FILE_NAME)?,
            display_unit: params.remove_optional(DISPLAY_UNIT)?,
            system_font: params.remove_optional(SYSTEM_FONT)?,
            use_mbcs: params.remove_optional(USE_MBCS)?,
            is_boc: params.remove_optional(IS_BOC)?,
            area_color: params.remove_optional::<i32>(AREA_COLOR)?.map(Color::new),
            styles: Vec::new(),
            file_version_info: params.remove_optional(FILE_VERSION_INFO)?,
        };

        let template_vault_guid = params.remove_with_default(TEMPLATE_VAULT_GUID, String::new())?;
        let template_item_guid = params.remove_with_default(TEMPLATE_ITEM_GUID, String::new())?;
        let template_revision_guid = params.remove_with_default(TEMPLATE_REVISION_GUID, String::new())?;
        let template_vault_hrid = params.remove_with_default(TEMPLATE_VAULT_HRID, String::new())?;
        let template_revision_hrid = params.remove_with_default(TEMPLATE_REVISION_HRID, String::new())?;
        let release_vault_guid = params.remove_with_default(RELEASE_VAULT_GUID, String::new())?;
        let release_item_guid = params.remove_with_default(RELEASE_ITEM_GUID, String::new())?;
        let item_revision_guid = params.remove_with_default(ITEM_REVISION_GUID, String::new())?;
        let props_vault_guid = params.remove_with_default(PROPS_VAULT_GUID, String::new())?;
        let props_revision_guid = params.remove_with_default(PROPS_REVISION_GUID, String::new())?;

        Ok(Self {
            base,
            fonts,
            display_settings,
            template_vault_guid,
            template_item_guid,
            template_revision_guid,
            template_vault_hrid,
            template_revision_hrid,
            release_vault_guid,
            release_item_guid,
            item_revision_guid,
            props_vault_guid,
            props_revision_guid,
        })
    }
}

/// Schematic template record (RECORD=39).
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchTemplate {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(key = FILE_NAME, default = String::new())]
    pub file_name: String,
}

/// Electrical wire record (RECORD=27).
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchWire {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = LINE_WIDTH, default = PenWidth::Zero)]
    pub line_width: PenWidth,
    #[param(key = LINE_STYLE, default = LineStyle::Solid)]
    pub line_style: LineStyle,
    #[param(indexed_coords, count_key = LOCATION_COUNT, x_prefix = "X", y_prefix = "Y")]
    pub vertices: Vec<CoordPoint>,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
    #[param(key = UNDERLINE_COLOR, default = Color::BLACK)]
    pub underline_color: Color,
    #[param(key = ASSIGNED_INTERFACE, default = String::new())]
    pub assigned_interface: String,
    #[param(key = ASSIGNED_INTERFACE_SIGNAL, default = String::new())]
    pub assigned_interface_signal: String,
}

/// Bus record (RECORD=26).
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchBus {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = LINE_WIDTH, default = PenWidth::Zero)]
    pub line_width: PenWidth,
    #[param(indexed_coords, count_key = LOCATION_COUNT, x_prefix = "X", y_prefix = "Y")]
    pub vertices: Vec<CoordPoint>,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
    #[param(key = UNDERLINE_COLOR, default = Color::BLACK)]
    pub underline_color: Color,
    #[param(key = ASSIGNED_INTERFACE, default = String::new())]
    pub assigned_interface: String,
    #[param(key = ASSIGNED_INTERFACE_SIGNAL, default = String::new())]
    pub assigned_interface_signal: String,
}

/// Net label record (RECORD=25).
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchNetLabel {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,
    #[param(key = ORIENTATION, default = RotationBy90::Rotate0)]
    pub orientation: RotationBy90,
    #[param(key = JUSTIFICATION, default = TextJustification::BottomLeft)]
    pub justification: TextJustification,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = FONT_ID, default = 1i32)]
    pub font_id: i32,
    #[param(key = TEXT, default = String::new())]
    pub text: String,
    #[param(key = IS_MIRRORED, default = false)]
    pub is_mirrored: bool,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}

/// Power object record (RECORD=17).
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchPowerObject {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = TEXT, default = String::new())]
    pub text: String,
    #[param(key = SYMBOL_TYPE, default = 0i32)]
    pub symbol_type: i32,
    #[param(key = STYLE, default = PowerObjectStyle::Circle)]
    pub style: PowerObjectStyle,
    #[param(key = SHOW_NET_NAME, default = true)]
    pub show_net_name: bool,
    #[param(key = ORIENTATION, default = RotationBy90::Rotate0)]
    pub orientation: RotationBy90,
    #[param(key = FONT_ID, default = 1i32)]
    pub font_id: i32,
    #[param(key = IS_CROSS_SHEET_CONNECTOR, default = false)]
    pub is_cross_sheet_connector: bool,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}

/// Port record (RECORD=18).
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchPort {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = AREA_COLOR, default = Color::BLACK)]
    pub area_color: Color,
    #[param(key = NAME, default = String::new())]
    pub name: String,
    #[param(key = IO_TYPE, default = PortIoType::Unspecified)]
    pub io_type: PortIoType,
    #[param(key = STYLE, default = PortArrowStyle::None)]
    pub style: PortArrowStyle,
    #[param(coord, key = WIDTH, frac_key = "Width_Frac")]
    pub width: Coord,
    #[param(coord, key = HEIGHT, frac_key = "Height_Frac")]
    pub height: Coord,
    #[param(key = TEXT_COLOR, default = Color::BLACK)]
    pub text_color: Color,
    #[param(key = FONT_ID, default = 1i32)]
    pub font_id: i32,
    #[param(key = ALIGNMENT, default = HorizontalAlign::Center)]
    pub alignment: HorizontalAlign,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
    #[param(key = HARNESS_TYPE, default = String::new())]
    pub harness_type: String,
    #[param(key = BORDER_WIDTH, default = PenWidth::Zero)]
    pub border_width: PenWidth,
    #[param(key = AUTO_SIZE, default = false)]
    pub auto_size: bool,
    #[param(key = PORT_NAME_IS_HIDDEN, default = false)]
    pub port_name_is_hidden: bool,
    #[param(key = OBJECT_DEFINITION_ID, default = String::new())]
    pub object_definition_id: String,
}

/// No-connect marker record (RECORD=22).
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchNoConnect {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = ORIENTATION, default = RotationBy90::Rotate0)]
    pub orientation: RotationBy90,
    #[param(key = SYMBOL, default = String::new())]
    pub symbol: String,
    #[param(key = IS_ACTIVE, default = true)]
    pub is_active: bool,
    #[param(key = SUPPRESS_ALL, default = true)]
    pub suppress_all: bool,
    #[param(key = ERROR_KIND_SET_TO_SUPPRESS, default = String::new())]
    pub error_kind_set_to_suppress: String,
    #[param(key = CONNECTION_PAIRS_TO_SUPPRESS, default = String::new())]
    pub connection_pairs_to_suppress: String,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}

/// Sheet name text record (RECORD=32).
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchSheetName {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,
    #[param(key = ORIENTATION, default = RotationBy90::Rotate0)]
    pub orientation: RotationBy90,
    #[param(key = JUSTIFICATION, default = TextJustification::BottomLeft)]
    pub justification: TextJustification,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = FONT_ID, default = 1i32)]
    pub font_id: i32,
    #[param(key = IS_HIDDEN, default = false)]
    pub is_hidden: bool,
    #[param(key = TEXT, default = String::new())]
    pub text: String,
    #[param(key = IS_MIRRORED, default = false)]
    pub is_mirrored: bool,
    #[param(key = NOT_AUTO_POSITION, default = false)]
    pub not_auto_position: bool,
    #[param(key = TEXT_HORZ_ANCHOR, default = TextHorzAnchor::None)]
    pub text_horz_anchor: TextHorzAnchor,
    #[param(key = TEXT_VERT_ANCHOR, default = TextVertAnchor::None)]
    pub text_vert_anchor: TextVertAnchor,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
    #[param(key = SELECTION, default = false)]
    pub selection: bool,
}

/// Sheet filename text record (RECORD=33).
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchSheetFileName {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,
    #[param(key = ORIENTATION, default = RotationBy90::Rotate0)]
    pub orientation: RotationBy90,
    #[param(key = JUSTIFICATION, default = TextJustification::BottomLeft)]
    pub justification: TextJustification,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = FONT_ID, default = 1i32)]
    pub font_id: i32,
    #[param(key = IS_HIDDEN, default = false)]
    pub is_hidden: bool,
    #[param(key = TEXT, default = String::new())]
    pub text: String,
    #[param(key = IS_MIRRORED, default = false)]
    pub is_mirrored: bool,
    #[param(key = NOT_AUTO_POSITION, default = false)]
    pub not_auto_position: bool,
    #[param(key = TEXT_HORZ_ANCHOR, default = TextHorzAnchor::None)]
    pub text_horz_anchor: TextHorzAnchor,
    #[param(key = TEXT_VERT_ANCHOR, default = TextVertAnchor::None)]
    pub text_vert_anchor: TextVertAnchor,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
    #[param(key = SELECTION, default = false)]
    pub selection: bool,
}

/// Junction record (RECORD=29).
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchJunction {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,
    #[param(key = SIZE, default = PenWidth::Zero)]
    pub size: PenWidth,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = LOCKED, default = true)]
    pub locked: bool,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}

/// Sheet symbol record (RECORD=15).
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchSheetSymbol {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,
    #[param(coord, key = X_SIZE, frac_key = "XSize_FRAC")]
    pub x_size: Coord,
    #[param(coord, key = Y_SIZE, frac_key = "YSize_FRAC")]
    pub y_size: Coord,
    #[param(key = LINE_WIDTH, default = PenWidth::Zero)]
    pub line_width: PenWidth,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = AREA_COLOR, default = Color::BLACK)]
    pub area_color: Color,
    #[param(key = IS_SOLID, default = false)]
    pub is_solid: bool,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
    #[param(key = SYMBOL_TYPE, default = SheetSymbolType::Normal)]
    pub symbol_type: SheetSymbolType,
    #[param(key = "SheetName", default = String::new())]
    pub sheet_name: String,
    #[param(key = FILE_NAME, default = String::new())]
    pub file_name: String,
    #[param(key = SHOW_HIDDEN_FIELDS, default = false)]
    pub show_hidden_fields: bool,
    #[param(key = DESIGN_ITEM_ID, default = String::new())]
    pub design_item_id: String,
    #[param(key = SOURCE_LIBRARY_NAME, default = String::new())]
    pub source_library_name: String,
    #[param(key = VAULT_GUID, default = String::new())]
    pub vault_guid: String,
    #[param(key = ITEM_GUID, default = String::new())]
    pub item_guid: String,
    #[param(key = REVISION_GUID, default = String::new())]
    pub revision_guid: String,
    #[param(key = REVISION_NAME, default = String::new())]
    pub revision_name: String,
}

/// Sheet entry record (RECORD=16).
///
/// Manual parsing required because `distance_from_top` uses a non-standard
/// fractional encoding (1_000_000 divisor with `_Frac`/`_Frac1` variants)
/// that the derive macro's `#[param(coord)]` cannot handle.
#[derive(Debug)]
pub(crate) struct SchSheetEntry {
    pub base: SchPrimitiveBase,
    pub location: CoordPoint,
    pub side: LeftRightSide,
    pub distance_from_top: Coord,
    pub color: Color,
    pub area_color: Color,
    pub text_color: Color,
    pub text_font_id: i32,
    pub text_style: String,
    pub name: String,
    pub harness_type: String,
    pub io_type: PortIoType,
    pub style: PortArrowStyle,
    pub arrow_kind: String,
    pub unique_id: String,
}

impl SchSheetEntry {
    pub(crate) fn from_params(params: &mut ParameterCollection) -> Result<Self> {
        let base = SchPrimitiveBase::from_params(params)?;
        let loc_x = params.remove_coord(LOCATION_X, LOCATION_X_FRAC)?;
        let loc_y = params.remove_coord(LOCATION_Y, LOCATION_Y_FRAC)?;
        let location = CoordPoint::new(loc_x, loc_y);
        let side: LeftRightSide = params.remove_with_default(SIDE, LeftRightSide::Left)?;
        let distance_from_top = params.remove_distance_from_top(DISTANCE_FROM_TOP)?;
        let color: Color = params.remove_with_default(COLOR, Color::BLACK)?;
        let area_color: Color = params.remove_with_default(AREA_COLOR, Color::BLACK)?;
        let text_color: Color = params.remove_with_default(TEXT_COLOR, Color::BLACK)?;
        let text_font_id: i32 = params.remove_with_default(TEXT_FONT_ID, 1i32)?;
        let text_style: String = params.remove_with_default(TEXT_STYLE, String::new())?;
        let name: String = params.remove_with_default(NAME, String::new())?;
        let harness_type: String = params.remove_with_default(HARNESS_TYPE, String::new())?;
        let io_type: PortIoType = params.remove_with_default(IO_TYPE, PortIoType::Unspecified)?;
        let style: PortArrowStyle = params.remove_with_default(STYLE, PortArrowStyle::None)?;
        let arrow_kind: String = params.remove_with_default(ARROW_KIND, String::new())?;
        let unique_id: String = params.remove_with_default(UNIQUE_ID, String::new())?;

        Ok(Self {
            base,
            location,
            side,
            distance_from_top,
            color,
            area_color,
            text_color,
            text_font_id,
            text_style,
            name,
            harness_type,
            io_type,
            style,
            arrow_kind,
            unique_id,
        })
    }

    pub(crate) fn to_params(&self, params: &mut ParameterCollection) {
        self.base.to_params(params);
        params.insert_coord_point(
            LOCATION_X, LOCATION_X_FRAC, LOCATION_Y, LOCATION_Y_FRAC, self.location,
        );
        if self.side != LeftRightSide::Left {
            params.insert(SIDE, self.side.to_param_value());
        }
        params.insert_distance_from_top(DISTANCE_FROM_TOP, self.distance_from_top);
        if self.color != Color::BLACK {
            params.insert(COLOR, self.color.to_param_value());
        }
        if self.area_color != Color::BLACK {
            params.insert(AREA_COLOR, self.area_color.to_param_value());
        }
        if self.text_color != Color::BLACK {
            params.insert(TEXT_COLOR, self.text_color.to_param_value());
        }
        if self.text_font_id != 1 {
            params.insert(TEXT_FONT_ID, self.text_font_id.to_param_value());
        }
        if !self.text_style.is_empty() {
            params.insert(TEXT_STYLE, self.text_style.to_param_value());
        }
        if !self.name.is_empty() {
            params.insert(NAME, self.name.to_param_value());
        }
        if !self.harness_type.is_empty() {
            params.insert(HARNESS_TYPE, self.harness_type.to_param_value());
        }
        if self.io_type != PortIoType::Unspecified {
            params.insert(IO_TYPE, self.io_type.to_param_value());
        }
        if self.style != PortArrowStyle::None {
            params.insert(STYLE, self.style.to_param_value());
        }
        if !self.arrow_kind.is_empty() {
            params.insert(ARROW_KIND, self.arrow_kind.to_param_value());
        }
        if !self.unique_id.is_empty() {
            params.insert(UNIQUE_ID, self.unique_id.to_param_value());
        }
    }
}

/// Bus entry record (RECORD=37).
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchBusEntry {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,
    #[param(coord_point, x_key = CORNER_X, x_frac = CORNER_X_FRAC, y_key = CORNER_Y, y_frac = CORNER_Y_FRAC)]
    pub corner: CoordPoint,
    #[param(key = LINE_WIDTH, default = PenWidth::Zero)]
    pub line_width: PenWidth,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
}

/// Parameter set record (RECORD=43).
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchParameterSet {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = ORIENTATION, default = RotationBy90::Rotate0)]
    pub orientation: RotationBy90,
    #[param(key = NAME, default = String::new())]
    pub name: String,
    #[param(key = STYLE, default = 0i32)]
    pub style: i32,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}

/// Note record (RECORD=209).
///
/// Field order matches Altium's `ExportNote` (FileFormatV5.cs:2372-2397).
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchNote {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,
    #[param(coord_point, x_key = CORNER_X, x_frac = CORNER_X_FRAC, y_key = CORNER_Y, y_frac = CORNER_Y_FRAC)]
    pub corner: CoordPoint,
    #[param(key = LINE_WIDTH, default = PenWidth::Zero)]
    pub line_width: PenWidth,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = AREA_COLOR, default = Color::BLACK)]
    pub area_color: Color,
    #[param(key = TEXT_COLOR, default = Color::BLACK)]
    pub text_color: Color,
    #[param(key = FONT_ID, default = 1i32)]
    pub font_id: i32,
    #[param(key = IS_SOLID, default = true)]
    pub is_solid: bool,
    #[param(key = SHOW_BORDER, default = true)]
    pub show_border: bool,
    #[param(key = ALIGNMENT, default = HorizontalAlign::Left)]
    pub alignment: HorizontalAlign,
    #[param(key = WORD_WRAP, default = true)]
    pub word_wrap: bool,
    #[param(key = CLIP_TO_RECT, default = true)]
    pub clip_to_rect: bool,
    #[param(key = TEXT, default = String::new())]
    pub text: String,
    #[param(coord, key = TEXT_MARGIN, frac_key = TEXT_MARGIN_FRAC)]
    pub text_margin: Coord,
    #[param(key = COLLAPSED, default = false)]
    pub collapsed: bool,
    #[param(key = AUTHOR, default = String::new())]
    pub author: String,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}

/// Probe record (RECORD=210).
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchProbe {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = ORIENTATION, default = RotationBy90::Rotate0)]
    pub orientation: RotationBy90,
    #[param(key = NAME, default = String::new())]
    pub name: String,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}

/// Compile mask record (RECORD=211).
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchCompileMask {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,
    #[param(coord_point, x_key = CORNER_X, x_frac = CORNER_X_FRAC, y_key = CORNER_Y, y_frac = CORNER_Y_FRAC)]
    pub corner: CoordPoint,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = AREA_COLOR, default = Color::BLACK)]
    pub area_color: Color,
    #[param(key = COLLAPSED, default = false)]
    pub collapsed: bool,
    #[param(key = LINE_WIDTH, default = PenWidth::Zero)]
    pub line_width: PenWidth,
}

/// Harness connector record (RECORD=215).
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchHarnessConnector {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,
    #[param(coord, key = X_SIZE, frac_key = "XSize_FRAC")]
    pub x_size: Coord,
    #[param(coord, key = Y_SIZE, frac_key = "YSize_FRAC")]
    pub y_size: Coord,
    #[param(key = LINE_WIDTH, default = PenWidth::Zero)]
    pub line_width: PenWidth,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = AREA_COLOR, default = Color::BLACK)]
    pub area_color: Color,
    #[param(key = PRIMARY_CONNECTION_POSITION, default = 1_000_000i32)]
    pub primary_connection_position: i32,
    #[param(key = HARNESS_CONNECTOR_SIDE, default = LeftRightSide::Left)]
    pub harness_connector_side: LeftRightSide,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}

/// Blanket/dashed rectangle record (RECORD=225).
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchBlanket {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,
    #[param(coord_point, x_key = CORNER_X, x_frac = CORNER_X_FRAC, y_key = CORNER_Y, y_frac = CORNER_Y_FRAC)]
    pub corner: CoordPoint,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = AREA_COLOR, default = Color::BLACK)]
    pub area_color: Color,
    #[param(key = LINE_STYLE, default = LineStyle::Dashed)]
    pub line_style: LineStyle,
    #[param(key = LINE_STYLE_EXT, default = LineStyle::Dashed)]
    pub line_style_ext: LineStyle,
    #[param(key = LINE_WIDTH, default = PenWidth::Zero)]
    pub line_width: PenWidth,
    #[param(indexed_coords, count_key = LOCATION_COUNT, x_prefix = "X", y_prefix = "Y")]
    pub vertices: Vec<CoordPoint>,
    #[param(key = COLLAPSED, default = false)]
    pub collapsed: bool,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}

// ── SchRecord dispatch enum ───────────────────────────────────────────────────

/// All implemented schematic record variants.
///
/// Variants are added incrementally as record types are implemented.
#[derive(Debug)]
pub(crate) enum SchRecord {
    Sheet(SchSheet),
    Template(SchTemplate),
    Wire(SchWire),
    Bus(SchBus),
    NetLabel(SchNetLabel),
    PowerObject(SchPowerObject),
    Port(SchPort),
    NoConnect(SchNoConnect),
    Junction(SchJunction),
    SheetName(SchSheetName),
    SheetFileName(SchSheetFileName),
    SheetSymbol(SchSheetSymbol),
    SheetEntry(SchSheetEntry),
    BusEntry(SchBusEntry),
    ParameterSet(SchParameterSet),
    Note(SchNote),
    Probe(SchProbe),
    CompileMask(SchCompileMask),
    Blanket(SchBlanket),
    Component(SchComponent),
    Pin(SchPin),
    Symbol(SchSymbol),
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
    Hyperlink(SchLabel),
    Designator(SchDesignator),
    Parameter(SchParameter),
    TextFrame(SchTextFrame),
    ImplementationList(SchImplementationList),
    Implementation(SchImplementation),
    ImplementationMap(SchImplementationMap),
    MapDefiner(SchMapDefiner),
    ParameterList(SchParameterList),
    HarnessConnector(SchHarnessConnector),
    HarnessEntry(SchSheetEntry),
    HarnessConnectorType(SchSheetName),
    SignalHarness(SchBus),
    HighLevelCodeSymbol(SchSheetSymbol),
    HighLevelCodeEntry(SchSheetEntry),
    HighLevelCodeName(SchSheetName),
    HighLevelCodeFileName(SchSheetFileName),
}

impl SchRecord {
    /// Get a reference to the unique_id field, if the variant has one.
    ///
    /// Returns `None` for variants without a unique_id: Sheet, Template,
    /// Symbol, Pie, ImplementationList, MapDefiner, ParameterList.
    pub(crate) fn unique_id(&self) -> Option<&str> {
        match self {
            // Variants WITHOUT unique_id
            Self::Sheet(_)
            | Self::Template(_)
            | Self::Symbol(_)
            | Self::Pie(_)
            | Self::ImplementationList(_)
            | Self::MapDefiner(_)
            | Self::ParameterList(_) => None,

            // Variants with unique_id on a named field
            Self::Wire(v) => Some(&v.unique_id),
            Self::Bus(v) => Some(&v.unique_id),
            Self::NetLabel(v) => Some(&v.unique_id),
            Self::PowerObject(v) => Some(&v.unique_id),
            Self::Port(v) => Some(&v.unique_id),
            Self::NoConnect(v) => Some(&v.unique_id),
            Self::Junction(v) => Some(&v.unique_id),
            Self::SheetName(v) => Some(&v.unique_id),
            Self::SheetFileName(v) => Some(&v.unique_id),
            Self::SheetSymbol(v) => Some(&v.unique_id),
            Self::SheetEntry(v) => Some(&v.unique_id),
            Self::BusEntry(v) => Some(&v.unique_id),
            Self::ParameterSet(v) => Some(&v.unique_id),
            Self::Note(v) => Some(&v.unique_id),
            Self::Probe(v) => Some(&v.unique_id),
            Self::CompileMask(v) => Some(&v.unique_id),
            Self::Blanket(v) => Some(&v.unique_id),
            Self::Component(v) => Some(&v.unique_id),
            Self::Pin(v) => Some(&v.unique_id),
            Self::Line(v) => Some(&v.unique_id),
            Self::Rectangle(v) => Some(&v.unique_id),
            Self::RoundRectangle(v) => Some(&v.unique_id),
            Self::Arc(v) => Some(&v.unique_id),
            Self::EllipticalArc(v) => Some(&v.unique_id),
            Self::Ellipse(v) => Some(&v.unique_id),
            Self::Polyline(v) => Some(&v.unique_id),
            Self::Polygon(v) => Some(&v.unique_id),
            Self::Bezier(v) => Some(&v.unique_id),
            Self::Image(v) => Some(&v.unique_id),
            Self::Label(v) => Some(&v.unique_id),
            Self::Hyperlink(v) => Some(&v.unique_id),
            Self::Designator(v) => Some(&v.unique_id),
            Self::Parameter(v) => Some(&v.unique_id),
            Self::TextFrame(v) => Some(&v.unique_id),
            Self::Implementation(v) => Some(&v.unique_id),
            Self::ImplementationMap(v) => Some(&v.unique_id),
            Self::HarnessConnector(v) => Some(&v.unique_id),
            Self::HarnessEntry(v) => Some(&v.unique_id),
            Self::HarnessConnectorType(v) => Some(&v.unique_id),
            Self::SignalHarness(v) => Some(&v.unique_id),
            Self::HighLevelCodeSymbol(v) => Some(&v.unique_id),
            Self::HighLevelCodeEntry(v) => Some(&v.unique_id),
            Self::HighLevelCodeName(v) => Some(&v.unique_id),
            Self::HighLevelCodeFileName(v) => Some(&v.unique_id),
        }
    }

    /// Get a mutable reference to the unique_id field, if the variant has one.
    pub(crate) fn unique_id_mut(&mut self) -> Option<&mut String> {
        match self {
            Self::Sheet(_)
            | Self::Template(_)
            | Self::Symbol(_)
            | Self::Pie(_)
            | Self::ImplementationList(_)
            | Self::MapDefiner(_)
            | Self::ParameterList(_) => None,

            Self::Wire(v) => Some(&mut v.unique_id),
            Self::Bus(v) => Some(&mut v.unique_id),
            Self::NetLabel(v) => Some(&mut v.unique_id),
            Self::PowerObject(v) => Some(&mut v.unique_id),
            Self::Port(v) => Some(&mut v.unique_id),
            Self::NoConnect(v) => Some(&mut v.unique_id),
            Self::Junction(v) => Some(&mut v.unique_id),
            Self::SheetName(v) => Some(&mut v.unique_id),
            Self::SheetFileName(v) => Some(&mut v.unique_id),
            Self::SheetSymbol(v) => Some(&mut v.unique_id),
            Self::SheetEntry(v) => Some(&mut v.unique_id),
            Self::BusEntry(v) => Some(&mut v.unique_id),
            Self::ParameterSet(v) => Some(&mut v.unique_id),
            Self::Note(v) => Some(&mut v.unique_id),
            Self::Probe(v) => Some(&mut v.unique_id),
            Self::CompileMask(v) => Some(&mut v.unique_id),
            Self::Blanket(v) => Some(&mut v.unique_id),
            Self::Component(v) => Some(&mut v.unique_id),
            Self::Pin(v) => Some(&mut v.unique_id),
            Self::Line(v) => Some(&mut v.unique_id),
            Self::Rectangle(v) => Some(&mut v.unique_id),
            Self::RoundRectangle(v) => Some(&mut v.unique_id),
            Self::Arc(v) => Some(&mut v.unique_id),
            Self::EllipticalArc(v) => Some(&mut v.unique_id),
            Self::Ellipse(v) => Some(&mut v.unique_id),
            Self::Polyline(v) => Some(&mut v.unique_id),
            Self::Polygon(v) => Some(&mut v.unique_id),
            Self::Bezier(v) => Some(&mut v.unique_id),
            Self::Image(v) => Some(&mut v.unique_id),
            Self::Label(v) => Some(&mut v.unique_id),
            Self::Hyperlink(v) => Some(&mut v.unique_id),
            Self::Designator(v) => Some(&mut v.unique_id),
            Self::Parameter(v) => Some(&mut v.unique_id),
            Self::TextFrame(v) => Some(&mut v.unique_id),
            Self::Implementation(v) => Some(&mut v.unique_id),
            Self::ImplementationMap(v) => Some(&mut v.unique_id),
            Self::HarnessConnector(v) => Some(&mut v.unique_id),
            Self::HarnessEntry(v) => Some(&mut v.unique_id),
            Self::HarnessConnectorType(v) => Some(&mut v.unique_id),
            Self::SignalHarness(v) => Some(&mut v.unique_id),
            Self::HighLevelCodeSymbol(v) => Some(&mut v.unique_id),
            Self::HighLevelCodeEntry(v) => Some(&mut v.unique_id),
            Self::HighLevelCodeName(v) => Some(&mut v.unique_id),
            Self::HighLevelCodeFileName(v) => Some(&mut v.unique_id),
        }
    }

    /// Get a reference to the base fields.
    ///
    /// Component and Pin have their own ownership fields rather than a nested
    /// `SchPrimitiveBase`, so this returns `None` for them.
    pub(crate) fn base(&self) -> Option<&SchPrimitiveBase> {
        match self {
            Self::Component(_) | Self::Pin(_) => None,
            Self::Sheet(v) => Some(&v.base),
            Self::Template(v) => Some(&v.base),
            Self::Wire(v) => Some(&v.base),
            Self::Bus(v) => Some(&v.base),
            Self::NetLabel(v) => Some(&v.base),
            Self::PowerObject(v) => Some(&v.base),
            Self::Port(v) => Some(&v.base),
            Self::NoConnect(v) => Some(&v.base),
            Self::Junction(v) => Some(&v.base),
            Self::SheetName(v) => Some(&v.base),
            Self::SheetFileName(v) => Some(&v.base),
            Self::SheetSymbol(v) => Some(&v.base),
            Self::SheetEntry(v) => Some(&v.base),
            Self::BusEntry(v) => Some(&v.base),
            Self::ParameterSet(v) => Some(&v.base),
            Self::Note(v) => Some(&v.base),
            Self::Probe(v) => Some(&v.base),
            Self::CompileMask(v) => Some(&v.base),
            Self::Blanket(v) => Some(&v.base),
            Self::Symbol(v) => Some(&v.base),
            Self::Line(v) => Some(&v.base),
            Self::Rectangle(v) => Some(&v.base),
            Self::RoundRectangle(v) => Some(&v.base),
            Self::Arc(v) => Some(&v.base),
            Self::EllipticalArc(v) => Some(&v.base),
            Self::Ellipse(v) => Some(&v.base),
            Self::Pie(v) => Some(&v.base),
            Self::Polyline(v) => Some(&v.base),
            Self::Polygon(v) => Some(&v.base),
            Self::Bezier(v) => Some(&v.base),
            Self::Image(v) => Some(&v.base),
            Self::Label(v) => Some(&v.base),
            Self::Hyperlink(v) => Some(&v.base),
            Self::Designator(v) => Some(&v.base),
            Self::Parameter(v) => Some(&v.base),
            Self::TextFrame(v) => Some(&v.base),
            Self::ImplementationList(v) => Some(&v.base),
            Self::Implementation(v) => Some(&v.base),
            Self::ImplementationMap(v) => Some(&v.base),
            Self::MapDefiner(v) => Some(&v.base),
            Self::ParameterList(v) => Some(&v.base),
            Self::HarnessConnector(v) => Some(&v.base),
            Self::HarnessEntry(v) => Some(&v.base),
            Self::HarnessConnectorType(v) => Some(&v.base),
            Self::SignalHarness(v) => Some(&v.base),
            Self::HighLevelCodeSymbol(v) => Some(&v.base),
            Self::HighLevelCodeEntry(v) => Some(&v.base),
            Self::HighLevelCodeName(v) => Some(&v.base),
            Self::HighLevelCodeFileName(v) => Some(&v.base),
        }
    }

    /// Get a mutable reference to the base fields.
    pub(crate) fn base_mut(&mut self) -> Option<&mut SchPrimitiveBase> {
        match self {
            Self::Component(_) | Self::Pin(_) => None,
            Self::Sheet(v) => Some(&mut v.base),
            Self::Template(v) => Some(&mut v.base),
            Self::Wire(v) => Some(&mut v.base),
            Self::Bus(v) => Some(&mut v.base),
            Self::NetLabel(v) => Some(&mut v.base),
            Self::PowerObject(v) => Some(&mut v.base),
            Self::Port(v) => Some(&mut v.base),
            Self::NoConnect(v) => Some(&mut v.base),
            Self::Junction(v) => Some(&mut v.base),
            Self::SheetName(v) => Some(&mut v.base),
            Self::SheetFileName(v) => Some(&mut v.base),
            Self::SheetSymbol(v) => Some(&mut v.base),
            Self::SheetEntry(v) => Some(&mut v.base),
            Self::BusEntry(v) => Some(&mut v.base),
            Self::ParameterSet(v) => Some(&mut v.base),
            Self::Note(v) => Some(&mut v.base),
            Self::Probe(v) => Some(&mut v.base),
            Self::CompileMask(v) => Some(&mut v.base),
            Self::Blanket(v) => Some(&mut v.base),
            Self::Symbol(v) => Some(&mut v.base),
            Self::Line(v) => Some(&mut v.base),
            Self::Rectangle(v) => Some(&mut v.base),
            Self::RoundRectangle(v) => Some(&mut v.base),
            Self::Arc(v) => Some(&mut v.base),
            Self::EllipticalArc(v) => Some(&mut v.base),
            Self::Ellipse(v) => Some(&mut v.base),
            Self::Pie(v) => Some(&mut v.base),
            Self::Polyline(v) => Some(&mut v.base),
            Self::Polygon(v) => Some(&mut v.base),
            Self::Bezier(v) => Some(&mut v.base),
            Self::Image(v) => Some(&mut v.base),
            Self::Label(v) => Some(&mut v.base),
            Self::Hyperlink(v) => Some(&mut v.base),
            Self::Designator(v) => Some(&mut v.base),
            Self::Parameter(v) => Some(&mut v.base),
            Self::TextFrame(v) => Some(&mut v.base),
            Self::ImplementationList(v) => Some(&mut v.base),
            Self::Implementation(v) => Some(&mut v.base),
            Self::ImplementationMap(v) => Some(&mut v.base),
            Self::MapDefiner(v) => Some(&mut v.base),
            Self::ParameterList(v) => Some(&mut v.base),
            Self::HarnessConnector(v) => Some(&mut v.base),
            Self::HarnessEntry(v) => Some(&mut v.base),
            Self::HarnessConnectorType(v) => Some(&mut v.base),
            Self::SignalHarness(v) => Some(&mut v.base),
            Self::HighLevelCodeSymbol(v) => Some(&mut v.base),
            Self::HighLevelCodeEntry(v) => Some(&mut v.base),
            Self::HighLevelCodeName(v) => Some(&mut v.base),
            Self::HighLevelCodeFileName(v) => Some(&mut v.base),
        }
    }
}

// ── SchLibComponent ───────────────────────────────────────────────────────────

/// A single component entry in a SchLib file, holding its component record and
/// all child records (pins, graphical primitives, text annotations, etc.).
pub(crate) struct SchLibComponent {
    pub component: SchComponent,
    pub records: Vec<SchRecord>,
    /// Records from the per-component Additional stream (separate from Data).
    pub additional_records: Vec<SchRecord>,
}

// ── Serialization ─────────────────────────────────────────────────────────────

/// Encodes the PinConglomerate bitmask byte from individual fields.
fn encode_pin_conglomerate(pin: &SchPin) -> u8 {
    let mut byte = pin.orientation as u8 & PIN_CONGLOMERATE_ORIENTATION_MASK;
    if pin.is_hidden {
        byte |= PIN_CONGLOMERATE_IS_HIDDEN;
    }
    if pin.show_name {
        byte |= PIN_CONGLOMERATE_SHOW_NAME;
    }
    if pin.show_designator {
        byte |= PIN_CONGLOMERATE_SHOW_DESIGNATOR;
    }
    if pin.is_not_accessible {
        byte |= PIN_CONGLOMERATE_NOT_ACCESSIBLE;
    }
    if pin.graphically_locked {
        byte |= PIN_CONGLOMERATE_GRAPHICALLY_LOCKED;
    }
    if pin.owner_index_additional_list {
        byte |= PIN_CONGLOMERATE_OWNER_INDEX_ADDITIONAL_LIST;
    }
    byte
}

/// Writes a DynamicString field truncated to at most 254 Windows-1252 bytes.
/// Matches Altium's `SchDataSerializerParam.Export_DynamicString` which truncates
/// DynamicString fields to 254 chars in binary mode. The full text is preserved
/// in PinDesc and PinWideText sidecar streams.
fn write_dynamic_string(w: &mut BinaryWriter, s: &str) {
    let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(s);
    let truncated = if encoded.len() > C_MAX_SHORT_STRING_LENGTH as usize { &encoded[..C_MAX_SHORT_STRING_LENGTH as usize] } else { &encoded };
    w.write_u8(truncated.len() as u8);
    w.write_bytes(truncated);
}

/// Serializes a SchPin back to the binary pin format (inverse of `parse_binary_pin`).
/// Returns the raw binary payload (not wrapped in a block).
pub(crate) fn serialize_binary_pin(pin: &SchPin) -> Result<Vec<u8>> {
    let mut w = BinaryWriter::new();

    w.write_u8(PIN_BINARY_CODE);
    w.write_i32_le(pin.owner_index);
    w.write_i16_le(pin.owner_part_id as i16);
    w.write_u8(pin.owner_part_display_mode);

    w.write_u8(pin.symbol_inner_edge as u8);
    w.write_u8(pin.symbol_outer_edge as u8);
    w.write_u8(pin.symbol_inside as u8);
    w.write_u8(pin.symbol_outside as u8);

    write_dynamic_string(&mut w, &pin.description);

    w.write_u8(pin.formal_type as u8);
    w.write_u8(pin.electrical as u8);
    w.write_u8(encode_pin_conglomerate(pin));

    w.write_i16_le((pin.pin_length.to_internal() / C_BASE_UNIT) as i16);
    w.write_i16_le((pin.location.x.to_internal() / C_BASE_UNIT) as i16);
    w.write_i16_le((pin.location.y.to_internal() / C_BASE_UNIT) as i16);

    w.write_i32_le(pin.color.raw());

    write_dynamic_string(&mut w, &pin.name);
    write_dynamic_string(&mut w, &pin.designator);
    w.write_pascal_string(&pin.swap_id_pin)?;
    w.write_pascal_string(&pin.swap_id_part)?;
    w.write_pascal_string(&pin.default_value)?;

    Ok(w.finish())
}

/// Serializes a SchComponent's fields into the given ParameterCollection.
/// Follows the exact parameter order from `FileFormatV5.cs` (see `parse_component_record`).
/// Caller is responsible for inserting the RECORD key before calling this.
pub(crate) fn serialize_component_record(comp: &SchComponent, params: &mut ParameterCollection) {
    params.insert(LIB_REFERENCE, comp.lib_reference.to_param_value());
    if comp.component_description != String::new() {
        params.insert(
            COMPONENT_DESCRIPTION,
            comp.component_description.to_param_value(),
        );
    }
    if comp.part_count != 0 {
        params.insert(PART_COUNT, comp.part_count.to_param_value());
    }
    if comp.display_mode_count != 0 {
        params.insert(DISPLAY_MODE_COUNT, comp.display_mode_count.to_param_value());
    }

    // ExportDataObject + ExportGraphicalObject fields (order matches Altium export)
    if comp.owner_index != 0 {
        params.insert(OWNER_INDEX, comp.owner_index.to_param_value());
    }
    if comp.is_not_accessible {
        params.insert(IS_NOT_ACCESSIBLE, comp.is_not_accessible.to_param_value());
    }
    if comp.index_in_sheet != 0 {
        params.insert(INDEX_IN_SHEET, comp.index_in_sheet.to_param_value());
    }
    if comp.owner_part_id != 0 {
        params.insert(OWNER_PART_ID, comp.owner_part_id.to_param_value());
    }
    if comp.owner_part_display_mode != 0 {
        params.insert(
            OWNER_PART_DISPLAY_MODE,
            comp.owner_part_display_mode.to_param_value(),
        );
    }
    if comp.graphically_locked {
        params.insert(GRAPHICALLY_LOCKED, comp.graphically_locked.to_param_value());
    }
    if comp.union_index != 0 {
        params.insert(UNION_INDEX, comp.union_index.to_param_value());
    }

    // Location (DXP frac coords) — T1: skip at zero
    if comp.location.x.to_internal() != 0 {
        params.insert_coord(LOCATION_X, LOCATION_X_FRAC, comp.location.x);
    }
    if comp.location.y.to_internal() != 0 {
        params.insert_coord(LOCATION_Y, LOCATION_Y_FRAC, comp.location.y);
    }

    if comp.display_mode != 0 {
        params.insert(DISPLAY_MODE, comp.display_mode.to_param_value());
    }
    if comp.is_mirrored {
        params.insert(IS_MIRRORED, comp.is_mirrored.to_param_value());
    }
    if comp.orientation != RotationBy90::Rotate0 {
        params.insert(ORIENTATION, comp.orientation.to_param_value());
    }
    if comp.current_part_id != 0 {
        params.insert(CURRENT_PART_ID, comp.current_part_id.to_param_value());
    }
    if comp.show_hidden_fields {
        params.insert(SHOW_HIDDEN_FIELDS, comp.show_hidden_fields.to_param_value());
    }
    if comp.show_hidden_pins {
        params.insert(SHOW_HIDDEN_PINS, comp.show_hidden_pins.to_param_value());
    }
    if !comp.library_path.is_empty() {
        params.insert(LIBRARY_PATH, comp.library_path.to_param_value());
    }
    if !comp.source_library_name.is_empty() {
        params.insert(
            SOURCE_LIBRARY_NAME,
            comp.source_library_name.to_param_value(),
        );
    }
    if !comp.database_table_name.is_empty() {
        params.insert(
            DATABASE_TABLE_NAME,
            comp.database_table_name.to_param_value(),
        );
    }
    if !comp.sheet_part_file_name.is_empty() {
        params.insert(
            SHEET_PART_FILE_NAME,
            comp.sheet_part_file_name.to_param_value(),
        );
    }
    if !comp.target_file_name.is_empty() {
        params.insert(TARGET_FILE_NAME, comp.target_file_name.to_param_value());
    }
    if !comp.unique_id.is_empty() {
        params.insert(UNIQUE_ID, comp.unique_id.to_param_value());
    }
    if comp.area_color != Color::BLACK {
        params.insert(AREA_COLOR, comp.area_color.to_param_value());
    }
    if comp.color != Color::BLACK {
        params.insert(COLOR, comp.color.to_param_value());
    }
    if comp.pin_color != Color::BLACK {
        params.insert(PIN_COLOR, comp.pin_color.to_param_value());
    }
    if comp.override_colors {
        params.insert(OVERIDE_COLORS, comp.override_colors.to_param_value());
    }
    if comp.display_field_names {
        params.insert(
            DISPLAY_FIELD_NAMES,
            comp.display_field_names.to_param_value(),
        );
    }
    if comp.designator_locked {
        params.insert(DESIGNATOR_LOCKED, comp.designator_locked.to_param_value());
    }
    // T2: always write (Export_Boolean_WithDefault)
    params.insert(PART_ID_LOCKED, comp.part_id_locked.to_param_value());
    if comp.pins_moveable {
        params.insert(PINS_MOVEABLE, comp.pins_moveable.to_param_value());
    }
    if !comp.alias_list.is_empty() {
        params.insert(ALIAS_LIST, comp.alias_list.to_param_value());
    }
    if comp.not_use_library_name {
        params.insert(
            NOT_USE_LIBRARY_NAME,
            comp.not_use_library_name.to_param_value(),
        );
    }
    if comp.not_use_db_table_name {
        params.insert(
            NOT_USE_DB_TABLE_NAME,
            comp.not_use_db_table_name.to_param_value(),
        );
    }
    if !comp.design_item_id.is_empty() {
        params.insert(DESIGN_ITEM_ID, comp.design_item_id.to_param_value());
    }
    if !comp.vault_guid.is_empty() {
        params.insert(VAULT_GUID, comp.vault_guid.to_param_value());
    }
    if !comp.item_guid.is_empty() {
        params.insert(ITEM_GUID, comp.item_guid.to_param_value());
    }
    if !comp.revision_guid.is_empty() {
        params.insert(REVISION_GUID, comp.revision_guid.to_param_value());
    }
    if !comp.symbol_vault_guid.is_empty() {
        params.insert(SYMBOL_VAULT_GUID, comp.symbol_vault_guid.to_param_value());
    }
    if !comp.symbol_item_guid.is_empty() {
        params.insert(SYMBOL_ITEM_GUID, comp.symbol_item_guid.to_param_value());
    }
    if !comp.symbol_revision_guid.is_empty() {
        params.insert(
            SYMBOL_REVISION_GUID,
            comp.symbol_revision_guid.to_param_value(),
        );
    }
    if !comp.generic_component_template_guid.is_empty() {
        params.insert(
            GENERIC_COMPONENT_TEMPLATE_GUID,
            comp.generic_component_template_guid.to_param_value(),
        );
    }
    if comp.has_only_current_part_info {
        params.insert(
            HAS_ONLY_CURRENT_PART_INFO,
            comp.has_only_current_part_info.to_param_value(),
        );
    }
    if comp.all_pin_count != 0 {
        params.insert(ALL_PIN_COUNT, comp.all_pin_count.to_param_value());
    }
    if !comp.key_component_unique_id.is_empty() {
        params.insert(
            KEY_COMPONENT_UNIQUE_ID,
            comp.key_component_unique_id.to_param_value(),
        );
    }
    if comp.component_kind != ComponentKind::Standard {
        params.insert(COMPONENT_KIND, comp.component_kind.to_param_value());
    }
    if comp.component_kind_version2 != ComponentKind::Standard {
        params.insert(
            COMPONENT_KIND_VERSION2,
            comp.component_kind_version2.to_param_value(),
        );
    }
    if comp.component_kind_version3 != ComponentKind::Standard {
        params.insert(
            COMPONENT_KIND_VERSION3,
            comp.component_kind_version3.to_param_value(),
        );
    }

    // CustomDisplayModeName0..N-1
    for (i, name) in comp.custom_display_mode_names.iter().enumerate() {
        if !name.is_empty() {
            let key = format!(
                "{}{}",
                altium_format_types::constants::component::CUSTOM_DISPLAY_MODE_NAME,
                i
            );
            params.insert(&key, name.to_param_value());
        }
    }
}

/// Serializes a SchMapDefiner's fields into the given ParameterCollection.
/// Caller is responsible for inserting the RECORD key before calling this.
pub(crate) fn serialize_map_definer(md: &SchMapDefiner, params: &mut ParameterCollection) {
    md.base.to_params(params);
    if !md.des_intf.is_empty() {
        params.insert(DES_INTF, md.des_intf.to_param_value());
    }
    if !md.des_imps.is_empty() {
        params.insert(DES_IMP_COUNT, (md.des_imps.len() as i32).to_param_value());
        for (i, imp) in md.des_imps.iter().enumerate() {
            let key = format!("DesImp{i}");
            if !imp.is_empty() {
                params.insert(&key, imp.to_param_value());
            }
        }
    }
}

/// Serializes a SchImplementation's fields into the given ParameterCollection.
/// Caller is responsible for inserting the RECORD key before calling this.
pub(crate) fn serialize_implementation(imp: &SchImplementation, params: &mut ParameterCollection) {
    imp.base.to_params(params);
    if !imp.description.is_empty() {
        params.insert(DESCRIPTION, imp.description.to_param_value());
    }
    if imp.use_component_library {
        params.insert(USE_COMPONENT_LIBRARY, imp.use_component_library.to_param_value());
    }
    if !imp.model_name.is_empty() {
        params.insert(MODEL_NAME, imp.model_name.to_param_value());
    }
    if !imp.model_type.is_empty() {
        params.insert(MODEL_TYPE, imp.model_type.to_param_value());
    }
    if !imp.datafile_links.is_empty() {
        params.insert(DATAFILE_COUNT, (imp.datafile_links.len() as i32).to_param_value());
        for (i, link) in imp.datafile_links.iter().enumerate() {
            if !link.location.is_empty() {
                params.insert(&format!("ModelDatafile{i}"), link.location.to_param_value());
            }
            if !link.entity_name.is_empty() {
                params.insert(&format!("ModelDatafileEntity{i}"), link.entity_name.to_param_value());
            }
            if !link.file_kind.is_empty() {
                params.insert(&format!("ModelDatafileKind{i}"), link.file_kind.to_param_value());
            }
        }
    }
    if !imp.model_vault_guid.is_empty() {
        params.insert(MODEL_VAULT_GUID, imp.model_vault_guid.to_param_value());
    }
    if !imp.model_item_guid.is_empty() {
        params.insert(MODEL_ITEM_GUID, imp.model_item_guid.to_param_value());
    }
    if !imp.model_revision_guid.is_empty() {
        params.insert(MODEL_REVISION_GUID, imp.model_revision_guid.to_param_value());
    }
    if imp.is_current {
        params.insert(IS_CURRENT, imp.is_current.to_param_value());
    }
    if imp.datalinks_locked {
        params.insert(DATALINKS_LOCKED, imp.datalinks_locked.to_param_value());
    }
    if imp.database_datalinks_locked {
        params.insert(DATABASE_DATALINKS_LOCKED, imp.database_datalinks_locked.to_param_value());
    }
    if imp.integrated_model {
        params.insert(INTEGRATED_MODEL, imp.integrated_model.to_param_value());
    }
    if imp.database_model {
        params.insert(DATABASE_MODEL, imp.database_model.to_param_value());
    }
    if !imp.unique_id.is_empty() {
        params.insert(UNIQUE_ID, imp.unique_id.to_param_value());
    }
    if !imp.model_location.is_empty() {
        params.insert(MODEL_LOCATION, imp.model_location.to_param_value());
    }
}

// Inserts RECORD (and RECORDEX for values >= 256) into a ParameterCollection.
fn insert_record_key(params: &mut ParameterCollection, record_type: SchRecordType) {
    let record_val = record_type as i32;
    if record_val >= 256 {
        params.insert(RECORD, "254".to_owned());
        params.insert(RECORD_EX, record_val.to_param_value());
    } else {
        params.insert(RECORD, record_val.to_param_value());
    }
}

/// Serializes any SchRecord into the appropriate block bytes (text or binary).
/// For text records: `|RECORD=N|field1=val1|...|field_n=val_n|\0` as a text block.
/// For binary records (Pin): binary payload as a binary block.
pub(crate) fn serialize_record(record: &SchRecord) -> Result<Vec<u8>> {
    match record {
        SchRecord::Pin(pin) => Ok(write_binary_block(&serialize_binary_pin(pin)?)),
        _ => {
            let mut params = ParameterCollection::new();
            insert_record_key(&mut params, record_type_for(record));
            fill_record_fields(record, &mut params)?;
            Ok(write_text_block(&params.to_bytes()))
        }
    }
}

// Returns the SchRecordType for any SchRecord variant.
fn record_type_for(record: &SchRecord) -> SchRecordType {
    match record {
        SchRecord::Sheet(_) => SchRecordType::Sheet,
        SchRecord::Template(_) => SchRecordType::Template,
        SchRecord::Wire(_) => SchRecordType::Wire,
        SchRecord::Bus(_) => SchRecordType::Bus,
        SchRecord::NetLabel(_) => SchRecordType::NetLabel,
        SchRecord::PowerObject(_) => SchRecordType::PowerObject,
        SchRecord::Port(_) => SchRecordType::Port,
        SchRecord::NoConnect(_) => SchRecordType::NoErc,
        SchRecord::Junction(_) => SchRecordType::Junction,
        SchRecord::SheetName(_) => SchRecordType::SheetName,
        SchRecord::SheetFileName(_) => SchRecordType::SheetFileName,
        SchRecord::SheetSymbol(_) => SchRecordType::SheetSymbol,
        SchRecord::SheetEntry(_) => SchRecordType::SheetEntry,
        SchRecord::BusEntry(_) => SchRecordType::BusEntry,
        SchRecord::ParameterSet(_) => SchRecordType::ParameterSet,
        SchRecord::Note(_) => SchRecordType::Note,
        SchRecord::Probe(_) => SchRecordType::Probe,
        SchRecord::CompileMask(_) => SchRecordType::CompileMask,
        SchRecord::Blanket(_) => SchRecordType::Blanket,
        SchRecord::Component(_) => SchRecordType::Component,
        SchRecord::Pin(_) => SchRecordType::Pin,
        SchRecord::Symbol(_) => SchRecordType::Symbol,
        SchRecord::Label(_) => SchRecordType::Label,
        SchRecord::Hyperlink(_) => SchRecordType::Hyperlink,
        SchRecord::Bezier(_) => SchRecordType::Bezier,
        SchRecord::Polyline(_) => SchRecordType::Polyline,
        SchRecord::Polygon(_) => SchRecordType::Polygon,
        SchRecord::Ellipse(_) => SchRecordType::Ellipse,
        SchRecord::Pie(_) => SchRecordType::Pie,
        SchRecord::RoundRectangle(_) => SchRecordType::RoundRectangle,
        SchRecord::EllipticalArc(_) => SchRecordType::EllipticalArc,
        SchRecord::Arc(_) => SchRecordType::Arc,
        SchRecord::Line(_) => SchRecordType::Line,
        SchRecord::Rectangle(_) => SchRecordType::Rectangle,
        SchRecord::TextFrame(_) => SchRecordType::TextFrame,
        SchRecord::Image(_) => SchRecordType::Image,
        SchRecord::Designator(_) => SchRecordType::Designator,
        SchRecord::Parameter(_) => SchRecordType::Parameter,
        SchRecord::ImplementationList(_) => SchRecordType::ImplementationList,
        SchRecord::Implementation(_) => SchRecordType::Implementation,
        SchRecord::ImplementationMap(_) => SchRecordType::ImplementationMap,
        SchRecord::MapDefiner(_) => SchRecordType::MapDefiner,
        SchRecord::ParameterList(_) => SchRecordType::ParameterList,
        SchRecord::HarnessConnector(_) => SchRecordType::HarnessConnector,
        SchRecord::HarnessEntry(_) => SchRecordType::HarnessEntry,
        SchRecord::HarnessConnectorType(_) => SchRecordType::HarnessConnectorType,
        SchRecord::SignalHarness(_) => SchRecordType::SignalHarness,
        SchRecord::HighLevelCodeSymbol(_) => SchRecordType::HighLevelCodeSymbol,
        SchRecord::HighLevelCodeEntry(_) => SchRecordType::HighLevelCodeEntry,
        SchRecord::HighLevelCodeName(_) => SchRecordType::HighLevelCodeName,
        SchRecord::HighLevelCodeFileName(_) => SchRecordType::HighLevelCodeFileName,
    }
}

// Fills field parameters into the collection (RECORD key already inserted).
fn fill_record_fields(record: &SchRecord, params: &mut ParameterCollection) -> Result<()> {
    match record {
        SchRecord::Sheet(_) => {
            return Err(AltiumFormatError::NotImplemented("SchSheet serialization".into()))
        }
        SchRecord::Template(v) => v.to_params(params),
        SchRecord::Wire(v) => v.to_params(params),
        SchRecord::Bus(v) => v.to_params(params),
        SchRecord::NetLabel(v) => v.to_params(params),
        SchRecord::PowerObject(v) => v.to_params(params),
        SchRecord::Port(v) => v.to_params(params),
        SchRecord::NoConnect(v) => v.to_params(params),
        SchRecord::Junction(v) => v.to_params(params),
        SchRecord::SheetName(v) => v.to_params(params),
        SchRecord::SheetFileName(v) => v.to_params(params),
        SchRecord::SheetSymbol(v) => v.to_params(params),
        SchRecord::SheetEntry(v) => v.to_params(params),
        SchRecord::BusEntry(v) => v.to_params(params),
        SchRecord::ParameterSet(v) => v.to_params(params),
        SchRecord::Note(v) => v.to_params(params),
        SchRecord::Probe(v) => v.to_params(params),
        SchRecord::CompileMask(v) => v.to_params(params),
        SchRecord::Blanket(v) => v.to_params(params),
        SchRecord::Component(v) => serialize_component_record(v, params),
        SchRecord::MapDefiner(v) => serialize_map_definer(v, params),
        SchRecord::Symbol(v) => v.to_params(params),
        SchRecord::Label(v) => v.to_params(params),
        SchRecord::Hyperlink(v) => v.to_params(params),
        SchRecord::Bezier(v) => v.to_params(params),
        SchRecord::Polyline(v) => v.to_params(params),
        SchRecord::Polygon(v) => v.to_params(params),
        SchRecord::Ellipse(v) => v.to_params(params),
        SchRecord::Pie(v) => v.to_params(params),
        SchRecord::RoundRectangle(v) => v.to_params(params),
        SchRecord::EllipticalArc(v) => v.to_params(params),
        SchRecord::Arc(v) => v.to_params(params),
        SchRecord::Line(v) => v.to_params(params),
        SchRecord::Rectangle(v) => v.to_params(params),
        SchRecord::TextFrame(v) => v.to_params(params),
        SchRecord::Image(v) => v.to_params(params),
        SchRecord::Designator(v) => v.to_params(params),
        SchRecord::Parameter(v) => v.to_params(params),
        SchRecord::ImplementationList(v) => v.to_params(params),
        SchRecord::Implementation(v) => serialize_implementation(v, params),
        SchRecord::ImplementationMap(v) => v.to_params(params),
        SchRecord::ParameterList(v) => v.to_params(params),
        SchRecord::HarnessConnector(v) => v.to_params(params),
        SchRecord::HarnessEntry(v) => v.to_params(params),
        SchRecord::HarnessConnectorType(v) => v.to_params(params),
        SchRecord::SignalHarness(v) => v.to_params(params),
        SchRecord::HighLevelCodeSymbol(v) => v.to_params(params),
        SchRecord::HighLevelCodeEntry(v) => v.to_params(params),
        SchRecord::HighLevelCodeName(v) => v.to_params(params),
        SchRecord::HighLevelCodeFileName(v) => v.to_params(params),
        SchRecord::Pin(_) => {
            return Err(AltiumFormatError::NotImplemented(
                "Pin serialization via fill_record_fields (use serialize_binary_pin)".into(),
            ))
        }
    }
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use altium_format_types::{
        IeeeSymbol, LineStyle, ParameterReadOnlyState, ParameterType, PenWidth, PinElectricalType,
        RotationBy90, TextJustification,
    };

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
        assert_eq!(base.owner_index, 0);
        assert!(!base.is_not_accessible);
        assert_eq!(base.index_in_sheet, 0);
        assert_eq!(base.owner_part_id, 0);
        assert_eq!(base.owner_part_display_mode, 0);
        assert!(!base.graphically_locked);
    }

    #[test]
    fn primitive_base_explicit_fields() {
        let mut params = pc(
            "|OwnerIndex=3|IsNotAccesible=T|OwnerPartId=2|OwnerPartDisplayMode=1|GraphicallyLocked=T|IndexInSheet=5|",
        );
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
        let mut params =
            pc("|Location.X=100|Location.X_Frac=50000|Location.Y=200|Location.Y_Frac=75000|");
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
            0,
            1,
            0, // owner_index, owner_part_id, owner_part_display_mode
            0,
            0,
            0,
            0,             // no symbols
            b"",           // empty description
            0,             // formal_type
            4,             // Passive
            0x00,          // conglomerate: Rotate0, no flags
            3,             // pin_length = 3 DXP units
            10,            // location_x = 10
            20,            // location_y = 20
            0x0000FF00i32, // green in BGR
            b"A1",         // name
            b"PA0",        // designator
            b"",
            b"",
            b"", // swap_id_pin, swap_id_part, default_value
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
        let data = make_pin_bytes(
            0, 1, 0, 0, 0, 0, 0, b"", 0, 4, cong, 1, 0, 0, 0, b"", b"", b"", b"", b"",
        );
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
            let data = make_pin_bytes(
                0, 1, 0, 0, 0, 0, 0, b"", 0, 4, raw, 1, 0, 0, 0, b"", b"", b"", b"", b"",
            );
            let pin = parse_binary_pin(&data).unwrap();
            assert_eq!(
                pin.orientation, expected,
                "orientation for conglomerate {raw}"
            );
        }
    }

    #[test]
    fn parse_binary_pin_coord_conversion() {
        // pin_length=5, x=100, y=-50 in DXP units
        let data = make_pin_bytes(
            0, 1, 0, 0, 0, 0, 0, b"", 0, 4, 0, 5, 100, -50, 0, b"", b"", b"", b"", b"",
        );
        let pin = parse_binary_pin(&data).unwrap();
        assert_eq!(pin.pin_length.to_internal(), 5 * 100_000);
        assert_eq!(pin.location.x.to_internal(), 100 * 100_000);
        assert_eq!(pin.location.y.to_internal(), -50 * 100_000);
    }

    #[test]
    fn parse_binary_pin_all_ieee_symbols() {
        for sym in 0u8..=36 {
            let data = make_pin_bytes(
                0, 1, 0, sym, sym, sym, sym, b"", 0, 4, 0, 1, 0, 0, 0, b"", b"", b"", b"", b"",
            );
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
            let data = make_pin_bytes(
                0, 1, 0, 0, 0, 0, 0, b"", 0, elec, 0, 1, 0, 0, 0, b"", b"", b"", b"", b"",
            );
            let pin = parse_binary_pin(&data).unwrap();
            let expected = PinElectricalType::try_from(elec).unwrap();
            assert_eq!(pin.electrical, expected);
        }
    }

    #[test]
    fn parse_binary_pin_invalid_binary_code() {
        let mut data = make_pin_bytes(
            0, 1, 0, 0, 0, 0, 0, b"", 0, 4, 0, 1, 0, 0, 0, b"", b"", b"", b"", b"",
        );
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
        let mut data = make_pin_bytes(
            0, 1, 0, 0, 0, 0, 0, b"", 0, 4, 0, 1, 0, 0, 0, b"", b"", b"", b"", b"",
        );
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
        let data = make_pin_bytes(
            0, 1, 0, 0, 0, 0, 0, b"", 0, 4, 0, 1, 0, 0, 0, b"", b"", b"", b"", b"",
        );
        let pin = parse_binary_pin(&data).unwrap();
        assert_eq!(pin.owner_index, 0);
        assert_eq!(pin.owner_part_id, 1);
        assert!(pin.swap_id_pin.is_empty());
        assert!(pin.swap_id_part.is_empty());
        assert!(pin.default_value.is_empty());
        assert_eq!(pin.pin_symbol_line_width, None);
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
        let mut params =
            pc("|Location.X=10|Location.Y=20|Corner.X=30|Corner.Y=40|LineWidth=1|LineStyle=2|");
        let line = SchLine::from_params(&mut params).unwrap();
        assert_eq!(line.location.x.to_internal(), 10 * 100_000);
        assert_eq!(line.location.y.to_internal(), 20 * 100_000);
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
        let mut params = pc(
            "|Location.X=0|Location.Y=0|Corner.X=50|Corner.Y=60|LineWidth=2|LineStyleExt=1|IsSolid=T|Transparent=F|",
        );
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
        let mut params =
            pc("|Location.X=5|Location.Y=5|Radius=100|StartAngle=45|EndAngle=270|LineWidth=1|");
        let arc = SchArc::from_params(&mut params).unwrap();
        assert_eq!(arc.radius.to_internal(), 100 * 100_000);
        assert_eq!(arc.start_angle, SchAngle(45.0));
        assert_eq!(arc.end_angle, Some(SchAngle(270.0)));
        assert_eq!(arc.line_width, PenWidth::Small);
    }

    #[test]
    fn arc_with_frac_radius() {
        let mut params = pc(
            "|Location.X=0|Location.Y=0|Radius=10|Radius_Frac=50000|StartAngle=0|EndAngle=360|LineWidth=0|",
        );
        let arc = SchArc::from_params(&mut params).unwrap();
        assert_eq!(arc.radius.to_internal(), 10 * 100_000 + 50_000);
    }

    #[test]
    fn elliptical_arc_parsed() {
        let mut params = pc(
            "|Location.X=0|Location.Y=0|Radius=50|SecondaryRadius=25|StartAngle=0|EndAngle=180|LineWidth=0|",
        );
        let ea = SchEllipticalArc::from_params(&mut params).unwrap();
        assert_eq!(ea.radius.to_internal(), 50 * 100_000);
        assert_eq!(ea.secondary_radius.to_internal(), 25 * 100_000);
        assert_eq!(ea.start_angle, SchAngle(0.0));
        assert_eq!(ea.end_angle, Some(SchAngle(180.0)));
    }

    #[test]
    fn ellipse_parsed() {
        let mut params = pc(
            "|Location.X=0|Location.Y=0|Radius=30|SecondaryRadius=20|LineWidth=0|IsSolid=T|Transparent=F|",
        );
        let el = SchEllipse::from_params(&mut params).unwrap();
        assert_eq!(el.radius.to_internal(), 30 * 100_000);
        assert_eq!(el.secondary_radius.to_internal(), 20 * 100_000);
        assert!(el.is_solid);
        assert!(!el.transparent);
    }

    #[test]
    fn pie_parsed() {
        let mut params = pc(
            "|Location.X=0|Location.Y=0|Radius=40|StartAngle=30|EndAngle=150|LineWidth=0|IsSolid=T|",
        );
        let pie = SchPie::from_params(&mut params).unwrap();
        assert_eq!(pie.radius.to_internal(), 40 * 100_000);
        assert_eq!(pie.start_angle, SchAngle(30.0));
        assert_eq!(pie.end_angle, Some(SchAngle(150.0)));
        assert!(pie.is_solid);
    }

    #[test]
    fn polyline_parsed() {
        let mut params = pc(
            "|Location.X=0|Location.Y=0|LocationCount=3|X1=1|Y1=2|X2=3|Y2=4|X3=5|Y3=6|LineWidth=1|LineStyle=0|LineShapeSize=0|StartLineShape=0|EndLineShape=0|",
        );
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
        let mut params = pc(
            "|Location.X=0|Location.Y=0|LocationCount=2|X1=10|Y1=20|X2=30|Y2=40|LineWidth=0|LineStyle=0|IsSolid=T|Transparent=F|",
        );
        let pg = SchPolygon::from_params(&mut params).unwrap();
        assert_eq!(pg.vertices.len(), 2);
        assert_eq!(pg.vertices[1].x.to_internal(), 30 * 100_000);
        assert!(pg.is_solid);
    }

    #[test]
    fn bezier_parsed() {
        let mut params = pc(
            "|Location.X=0|Location.Y=0|LocationCount=4|X1=0|Y1=0|X2=10|Y2=20|X3=30|Y3=20|X4=40|Y4=0|LineWidth=1|LineStyle=0|",
        );
        let bz = SchBezier::from_params(&mut params).unwrap();
        assert_eq!(bz.vertices.len(), 4);
        assert_eq!(bz.vertices[3].x.to_internal(), 40 * 100_000);
        assert_eq!(bz.line_width, PenWidth::Small);
    }

    #[test]
    fn image_parsed() {
        let mut params = pc(
            "|Location.X=0|Location.Y=0|Corner.X=100|Corner.Y=80|EmbedImage=T|FileName=test.bmp|KeepAspect=T|",
        );
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
        let mut params = pc(
            "|Location.X=10|Location.Y=20|Text=Hello|FontID=2|Justification=3|Orientation=1|IsMirrored=T|",
        );
        let label = SchLabel::from_params(&mut params).unwrap();
        assert_eq!(label.location.x.to_internal(), 10 * 100_000);
        assert_eq!(label.text, "Hello");
        assert_eq!(label.font_id, 2);
        assert_eq!(label.justification, TextJustification::CenterLeft);
        assert_eq!(label.orientation, RotationBy90::Rotate90);
        assert!(label.is_mirrored);
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
    }

    #[test]
    fn designator_parsed() {
        let mut params = pc(
            "|Location.X=5|Location.Y=5|Text=U1|Name=Designator|FontID=1|UniqueID=ABCDEF|ReadOnlyState=1|IsHidden=F|Orientation=0|IsMirrored=F|Justification=0|NotAutoPosition=F|",
        );
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
        let mut params = pc(
            "|Location.X=0|Location.Y=0|Text=100nF|Name=Comment|FontID=1|UniqueID=XYZ|ReadOnlyState=0|IsHidden=F|Orientation=0|IsMirrored=F|Justification=0|NotAutoPosition=F|ShowName=T|ParamType=0|",
        );
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
        assert_eq!(param.text, "");
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
        let mut params = pc(
            "|Location.X=0|Location.Y=0|Corner.X=200|Corner.Y=100|Text=Hello World|FontID=2|Alignment=1|WordWrap=T|IsSolid=F|LineWidth=1|TextMargin=5|ShowBorder=T|Transparent=T|ClipToRect=F|",
        );
        let tf = SchTextFrame::from_params(&mut params).unwrap();
        assert_eq!(tf.corner.x.to_internal(), 200 * 100_000);
        assert_eq!(tf.text, "Hello World");
        assert_eq!(tf.font_id, 2);
        assert_eq!(tf.alignment, HorizontalAlign::Left);
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
        assert_eq!(tf.alignment, HorizontalAlign::Left);
        assert!(!tf.word_wrap); // parse default: false (absent = false)
        assert_eq!(tf.line_width, PenWidth::Zero); // C# default: eZeroSize
        assert!(!tf.show_border); // C# default: false
        assert!(!tf.transparent); // C# default: false
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
        let mut params = pc(
            "|ModelName=SOIC127P600X175-8N|ModelType=PCBLIB|Description=SOP-8|IsCurrent=T|DatalinksLocked=F|DatabaseDatalinksLocked=F|IntegratedModel=F|DatabaseModel=F|UniqueID=ABC123|DatafileCount=1|ModelDatafile0=Lib.PcbLib|ModelDatafileEntity0=SOIC127P600X175-8N|ModelDatafileKind0=PCBLIB|UseComponentLibrary=F|",
        );
        let imp = SchImplementation::from_params(&mut params).unwrap();
        assert_eq!(imp.model_name, "SOIC127P600X175-8N");
        assert_eq!(imp.model_type, "PCBLIB");
        assert_eq!(imp.description, "SOP-8");
        assert!(imp.is_current);
        assert!(!imp.datalinks_locked);
        assert!(!imp.integrated_model);
        assert_eq!(imp.unique_id, "ABC123");
        assert_eq!(imp.datafile_links.len(), 1);
        assert_eq!(imp.datafile_links[0].location, "Lib.PcbLib");
        assert_eq!(imp.datafile_links[0].entity_name, "SOIC127P600X175-8N");
        assert_eq!(imp.datafile_links[0].file_kind, "PCBLIB");
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
        assert!(imp.datafile_links.is_empty());
    }

    #[test]
    fn implementation_map_parses() {
        let mut params = pc("|OwnerIndex=3|UniqueID=ABC123|");
        let im = SchImplementationMap::from_params(&mut params).unwrap();
        assert_eq!(im.base.owner_index, 3);
        assert_eq!(im.unique_id, "ABC123");
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

    // ── ToParams roundtrip tests ─────────────────────────────────────────────

    #[test]
    fn primitive_base_roundtrip_explicit_fields() {
        let mut params = pc(
            "|OwnerIndex=3|IsNotAccesible=T|OwnerPartId=2|OwnerPartDisplayMode=1|GraphicallyLocked=T|IndexInSheet=5|",
        );
        let base = SchPrimitiveBase::from_params(&mut params).unwrap();
        params.assert_exhausted().unwrap();

        let mut out = ParameterCollection::new();
        base.to_params(&mut out);
        let bytes = out.to_bytes();

        let mut rt = ParameterCollection::from_bytes(&bytes).unwrap();
        let base2 = SchPrimitiveBase::from_params(&mut rt).unwrap();
        rt.assert_exhausted().unwrap();

        assert_eq!(base2.owner_index, 3);
        assert!(base2.is_not_accessible);
        assert_eq!(base2.owner_part_id, 2);
        assert_eq!(base2.owner_part_display_mode, 1);
        assert!(base2.graphically_locked);
        assert_eq!(base2.index_in_sheet, 5);
    }

    #[test]
    fn primitive_base_t1_skips_defaults() {
        // All defaults: owner_index=0, is_not_accessible=false, etc.
        let mut params = pc("|");
        let base = SchPrimitiveBase::from_params(&mut params).unwrap();

        let mut out = ParameterCollection::new();
        base.to_params(&mut out);
        let bytes = out.to_bytes();

        // T1 skips all default values, so output should be just "\0"
        // (the base only has WithDefault fields, all at their defaults)
        let mut rt = ParameterCollection::from_bytes(&bytes).unwrap();
        let base2 = SchPrimitiveBase::from_params(&mut rt).unwrap();
        rt.assert_exhausted().unwrap();
        assert_eq!(base2.owner_index, 0);
        assert!(!base2.is_not_accessible);
    }

    #[test]
    fn line_roundtrip() {
        let mut params = pc(
            "|OwnerIndex=0|Location.X=10|Location.Y=20|Corner.X=30|Corner.Y=40|LineWidth=1|LineStyle=2|LineStyleExt=1|UniqueID=ABC|",
        );
        let line = SchLine::from_params(&mut params).unwrap();
        params.assert_exhausted().unwrap();

        let mut out = ParameterCollection::new();
        line.to_params(&mut out);
        let bytes = out.to_bytes();

        let mut rt = ParameterCollection::from_bytes(&bytes).unwrap();
        let line2 = SchLine::from_params(&mut rt).unwrap();
        rt.assert_exhausted().unwrap();

        assert_eq!(line2.base.owner_index, 0);
        assert_eq!(line2.location.x.to_internal(), 10 * 100_000);
        assert_eq!(line2.location.y.to_internal(), 20 * 100_000);
        assert_eq!(line2.corner.x.to_internal(), 30 * 100_000);
        assert_eq!(line2.corner.y.to_internal(), 40 * 100_000);
        assert_eq!(line2.line_width, PenWidth::Small);
        assert_eq!(line2.line_style, LineStyle::Dotted);
        assert_eq!(line2.line_style_ext, LineStyle::Dashed);
        assert_eq!(line2.unique_id, "ABC");
    }

    #[test]
    fn polyline_roundtrip() {
        let mut params = pc("|LocationCount=3|X1=1|Y1=2|X2=3|Y2=4|X3=5|Y3=6|LineWidth=1|");
        let pl = SchPolyline::from_params(&mut params).unwrap();
        params.assert_exhausted().unwrap();

        let mut out = ParameterCollection::new();
        pl.to_params(&mut out);
        let bytes = out.to_bytes();

        let mut rt = ParameterCollection::from_bytes(&bytes).unwrap();
        let pl2 = SchPolyline::from_params(&mut rt).unwrap();
        rt.assert_exhausted().unwrap();

        assert_eq!(pl2.vertices.len(), 3);
        assert_eq!(pl2.vertices[0].x.to_internal(), 1 * 100_000);
        assert_eq!(pl2.vertices[2].y.to_internal(), 6 * 100_000);
        assert_eq!(pl2.line_width, PenWidth::Small);
    }

    #[test]
    fn arc_with_frac_roundtrip() {
        let mut params = pc(
            "|Location.X=5|Location.X_Frac=25000|Location.Y=10|Radius=100|Radius_Frac=50000|StartAngle=45|EndAngle=270|LineWidth=2|",
        );
        let arc = SchArc::from_params(&mut params).unwrap();
        params.assert_exhausted().unwrap();

        let mut out = ParameterCollection::new();
        arc.to_params(&mut out);
        let bytes = out.to_bytes();

        let mut rt = ParameterCollection::from_bytes(&bytes).unwrap();
        let arc2 = SchArc::from_params(&mut rt).unwrap();
        rt.assert_exhausted().unwrap();

        assert_eq!(arc2.location.x.to_internal(), 5 * 100_000 + 25_000);
        assert_eq!(arc2.radius.to_internal(), 100 * 100_000 + 50_000);
        assert_eq!(arc2.start_angle, SchAngle(45.0));
        assert_eq!(arc2.end_angle, Some(SchAngle(270.0)));
    }

    #[test]
    fn implementation_roundtrip() {
        let mut params = pc(
            "|ModelName=SOIC|ModelType=PCBLIB|Description=SOP-8|IsCurrent=T|DatafileCount=1|ModelDatafile0=Lib.PcbLib|ModelDatafileEntity0=SOIC|ModelDatafileKind0=PCBLIB|",
        );
        let imp = SchImplementation::from_params(&mut params).unwrap();
        params.assert_exhausted().unwrap();

        let mut out = ParameterCollection::new();
        serialize_implementation(&imp, &mut out);
        let bytes = out.to_bytes();

        let mut rt = ParameterCollection::from_bytes(&bytes).unwrap();
        let imp2 = SchImplementation::from_params(&mut rt).unwrap();
        rt.assert_exhausted().unwrap();

        assert_eq!(imp2.model_name, "SOIC");
        assert_eq!(imp2.model_type, "PCBLIB");
        assert_eq!(imp2.description, "SOP-8");
        assert!(imp2.is_current);
        assert_eq!(imp2.datafile_links.len(), 1);
        assert_eq!(imp2.datafile_links[0].location, "Lib.PcbLib");
    }

    #[test]
    fn implementation_multi_datafile() {
        let mut params = pc(
            "|ModelName=DUAL|ModelType=PCBLIB|DatafileCount=2|ModelDatafile0=A.PcbLib|ModelDatafileEntity0=FP_A|ModelDatafileKind0=PCBLIB|ModelDatafile1=B.PcbLib|ModelDatafileEntity1=FP_B|ModelDatafileKind1=PCBLIB|",
        );
        let imp = SchImplementation::from_params(&mut params).unwrap();
        params.assert_exhausted().unwrap();

        assert_eq!(imp.datafile_links.len(), 2);
        assert_eq!(imp.datafile_links[0].location, "A.PcbLib");
        assert_eq!(imp.datafile_links[0].entity_name, "FP_A");
        assert_eq!(imp.datafile_links[1].location, "B.PcbLib");
        assert_eq!(imp.datafile_links[1].entity_name, "FP_B");

        // Roundtrip
        let mut out = ParameterCollection::new();
        serialize_implementation(&imp, &mut out);
        let bytes = out.to_bytes();

        let mut rt = ParameterCollection::from_bytes(&bytes).unwrap();
        let imp2 = SchImplementation::from_params(&mut rt).unwrap();
        rt.assert_exhausted().unwrap();

        assert_eq!(imp2.datafile_links.len(), 2);
        assert_eq!(imp2.datafile_links[0].location, "A.PcbLib");
        assert_eq!(imp2.datafile_links[1].location, "B.PcbLib");
    }

    // ── serialize_binary_pin roundtrip tests ──────────────────────────

    fn make_test_pin() -> SchPin {
        SchPin {
            owner_index: 0,
            owner_part_id: 1,
            owner_part_display_mode: 0,
            symbol_inner_edge: IeeeSymbol::NoSymbol,
            symbol_outer_edge: IeeeSymbol::NoSymbol,
            symbol_inside: IeeeSymbol::NoSymbol,
            symbol_outside: IeeeSymbol::NoSymbol,
            symbol_inner_edge_present: true,
            symbol_outer_edge_present: true,
            symbol_inside_present: true,
            symbol_outside_present: true,
            symbol: None,
            description: String::new(),
            formal_type: StdLogicState::Uninitialized,
            electrical: PinElectricalType::Input,
            orientation: RotationBy90::Rotate0,
            is_hidden: false,
            show_name: true,
            show_designator: true,
            is_not_accessible: false,
            graphically_locked: false,
            owner_index_additional_list: false,
            pin_length: Coord::from_internal(3 * 100_000),
            location: CoordPoint::new(
                Coord::from_internal(10 * 100_000),
                Coord::from_internal(20 * 100_000),
            ),
            color: Color::new(0x00800000),
            name: "A0".to_owned(),
            designator: "1".to_owned(),
            swap_id_pin: String::new(),
            swap_id_part: String::new(),
            swap_id_pair: String::new(),
            default_value: String::new(),
            spice_pin_name: String::new(),
            hidden_net_name: String::new(),
            unique_id: String::new(),
            // Sidecar fields (not serialized in binary pin format)
            pin_symbol_line_width: None,
            pin_package_length: String::new(),
            propagation_delay: String::new(),
            selected_functions: Vec::new(),
            defined_functions: Vec::new(),
            name_text_data: None,
            designator_text_data: None,
        }
    }

    #[test]
    fn serialize_binary_pin_roundtrip_minimal() {
        let pin = make_test_pin();
        let data = serialize_binary_pin(&pin).unwrap();
        let pin2 = parse_binary_pin(&data).unwrap();
        assert_eq!(pin2.owner_index, pin.owner_index);
        assert_eq!(pin2.owner_part_id, pin.owner_part_id);
        assert_eq!(pin2.electrical, pin.electrical);
        assert_eq!(pin2.pin_length, pin.pin_length);
        assert_eq!(pin2.location, pin.location);
        assert_eq!(pin2.color, pin.color);
        assert_eq!(pin2.name, pin.name);
        assert_eq!(pin2.designator, pin.designator);
        assert_eq!(pin2.is_hidden, pin.is_hidden);
        assert_eq!(pin2.show_name, pin.show_name);
        assert_eq!(pin2.show_designator, pin.show_designator);
    }

    #[test]
    fn serialize_binary_pin_roundtrip_with_flags() {
        let mut pin = make_test_pin();
        pin.owner_index = 5;
        pin.owner_part_id = 2;
        pin.owner_part_display_mode = 1;
        pin.symbol_inner_edge = IeeeSymbol::Clock;
        pin.symbol_outer_edge = IeeeSymbol::Dot;
        pin.description = "Test pin description".to_owned();
        pin.formal_type = StdLogicState::ForcingUnknown;
        pin.electrical = PinElectricalType::Output;
        pin.orientation = RotationBy90::Rotate90;
        pin.is_hidden = true;
        pin.show_name = false;
        pin.show_designator = false;
        pin.is_not_accessible = true;
        pin.graphically_locked = true;
        pin.pin_length = Coord::from_internal(5 * 100_000);
        pin.location =
            CoordPoint::new(Coord::from_internal(-10 * 100_000), Coord::from_internal(0));
        pin.color = Color::new(0x000000FF);
        pin.name = "DATA".to_owned();
        pin.designator = "2".to_owned();
        pin.swap_id_pin = "swap1".to_owned();
        pin.swap_id_part = "swapP".to_owned();
        pin.default_value = "HIGH".to_owned();

        let data = serialize_binary_pin(&pin).unwrap();
        let pin2 = parse_binary_pin(&data).unwrap();
        assert_eq!(pin2.owner_index, 5);
        assert_eq!(pin2.owner_part_id, 2);
        assert_eq!(pin2.owner_part_display_mode, 1);
        assert_eq!(pin2.symbol_inner_edge, IeeeSymbol::Clock);
        assert_eq!(pin2.symbol_outer_edge, IeeeSymbol::Dot);
        assert_eq!(pin2.description, "Test pin description");
        assert_eq!(pin2.formal_type, StdLogicState::ForcingUnknown);
        assert_eq!(pin2.electrical, PinElectricalType::Output);
        assert_eq!(pin2.orientation, RotationBy90::Rotate90);
        assert!(pin2.is_hidden);
        assert!(!pin2.show_name);
        assert!(!pin2.show_designator);
        assert!(pin2.is_not_accessible);
        assert!(pin2.graphically_locked);
        assert_eq!(pin2.pin_length, Coord::from_internal(5 * 100_000));
        assert_eq!(pin2.location.x, Coord::from_internal(-10 * 100_000));
        assert_eq!(pin2.name, "DATA");
        assert_eq!(pin2.designator, "2");
        assert_eq!(pin2.swap_id_pin, "swap1");
        assert_eq!(pin2.swap_id_part, "swapP");
        assert_eq!(pin2.default_value, "HIGH");
    }

    // ── serialize_record roundtrip tests ─────────────────────────────

    #[test]
    fn serialize_record_line_roundtrip() {
        use crate::block_stream::{BlockFormat, parse_blocks};

        let mut params = pc(
            "|OwnerIndex=0|OwnerPartId=1|LineWidth=1|Color=128|Location.X=10|Location.Y=20|Corner.X=30|Corner.Y=40|",
        );
        let line = SchLine::from_params(&mut params).unwrap();
        params.assert_exhausted().unwrap();

        let record = SchRecord::Line(line);
        let bytes = serialize_record(&record).unwrap();
        let blocks = parse_blocks(&bytes).unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].format, BlockFormat::Text);

        // Re-parse: strip the RECORD key and parse back
        let mut rt_params = ParameterCollection::from_bytes(&blocks[0].data).unwrap();
        let record_val: i32 = rt_params.remove_required("RECORD").unwrap();
        assert_eq!(record_val, 13); // SchRecordType::Line
        let line2 = SchLine::from_params(&mut rt_params).unwrap();
        rt_params.assert_exhausted().unwrap();
        assert_eq!(line2.location.x, Coord::from_dxp_frac(10, 0));
        assert_eq!(line2.location.y, Coord::from_dxp_frac(20, 0));
        assert_eq!(line2.corner.x, Coord::from_dxp_frac(30, 0));
        assert_eq!(line2.corner.y, Coord::from_dxp_frac(40, 0));
        assert_eq!(line2.color, Color::new(128));
    }

    #[test]
    fn serialize_record_pin_roundtrip() {
        use crate::block_stream::{BlockFormat, parse_blocks};

        let mut pin = make_test_pin();
        pin.electrical = PinElectricalType::Passive;
        pin.orientation = RotationBy90::Rotate180;
        pin.pin_length = Coord::from_internal(2 * 100_000);
        pin.location = CoordPoint::new(
            Coord::from_internal(5 * 100_000),
            Coord::from_internal(-3 * 100_000),
        );
        pin.name = "GND".to_owned();
        pin.designator = "4".to_owned();

        let record = SchRecord::Pin(pin);
        let bytes = serialize_record(&record).unwrap();
        let blocks = parse_blocks(&bytes).unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].format, BlockFormat::Binary);

        let pin2 = parse_binary_pin(&blocks[0].data).unwrap();
        assert_eq!(pin2.name, "GND");
        assert_eq!(pin2.designator, "4");
        assert_eq!(pin2.electrical, PinElectricalType::Passive);
        assert_eq!(pin2.orientation, RotationBy90::Rotate180);
        assert_eq!(pin2.pin_length, Coord::from_internal(2 * 100_000));
        assert_eq!(pin2.location.x, Coord::from_internal(5 * 100_000));
        assert_eq!(pin2.location.y, Coord::from_internal(-3 * 100_000));
    }
}
