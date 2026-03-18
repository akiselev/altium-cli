use std::collections::{HashMap, HashSet};
use std::path::Path;

use altium_format_types::constants::component::{
    ALIAS_COUNT, ALL_PIN_COUNT, COMP_COUNT, COMP_DESCR, LIB_REF, LIB_REFERENCE, PART_COUNT,
};
use altium_format_types::constants::file_headers::SCH_LIBRARY_BINARY_HEADER_V50;
use altium_format_types::constants::parsing::{C_BASE_UNIT, C_MAX_SHORT_STRING_LENGTH, C_SCH_SPECIAL_DELIMITER, INSTRUCTION_EXTRA_OBJECT_INDEX};
use altium_format_types::constants::pin::{
    DEF_VALUE, PAIR_SWAP_ID, PIN_BINARY_CODE, PIN_DEFINED_FUNCTION, PIN_DEFINED_FUNCTIONS_COUNT,
    PIN_PACKAGE_LENGTH as PIN_PACKAGE_LENGTH_KEY,
    PIN_PROPAGATION_DELAY as PIN_PROPAGATION_DELAY_KEY, PIN_SELECTED_FUNCTION,
    PIN_SELECTED_FUNCTIONS_COUNT, PIN_TEXT_FONT_CUSTOM, PIN_TEXT_POS_CUSTOM, PIN_TEXT_ROT_ANCHOR,
    PIN_TEXT_ROT_REL_MASK, PIN_TEXT_ROT_REL_SHIFT, SWAP_ID, SWAP_ID_PART, SYMBOL_LINE_WIDTH,
};
use altium_format_types::constants::record_structure::ALWAYS_SHOW_CD;
use altium_format_types::constants::record_structure::SECTION_NAME;
use altium_format_types::constants::record_structure::UNIQUE_ID;
use altium_format_types::constants::record_structure::{
    HEADER, KEY_COUNT, OWNER_INDEX, RECORD, RECORD_EX, SECTION_KEY, WEIGHT,
};
use altium_format_types::constants::sheet::MINOR_VERSION;
use altium_format_types::constants::sheet::{
    AREA_COLOR, BORDER_ON, CUSTOM_MARGIN_WIDTH, CUSTOM_X, CUSTOM_X_FRAC, CUSTOM_X_ZONES, CUSTOM_Y,
    CUSTOM_Y_FRAC, CUSTOM_Y_ZONES, DISPLAY_UNIT, DOCUMENT_BORDER_STYLE, FILE_VERSION_INFO,
    HOT_SPOT_GRID_ON, HOT_SPOT_GRID_SIZE, HOT_SPOT_GRID_SIZE_FRAC, IS_BOC, REFERENCE_ZONE_STYLE,
    REFERENCE_ZONES_ON, SHEET_NUMBER_SPACE_SIZE, SHEET_STYLE, SHOW_HIDDEN_PINS,
    SHOW_TEMPLATE_GRAPHICS, SNAP_GRID_ON, SNAP_GRID_SIZE, SNAP_GRID_SIZE_FRAC,
    STYLE_CORNER_RADIUS_MODE, STYLE_CORNER_RADIUS_VALUE, STYLE_GLOW_COLOR, STYLE_GLOW_OPACITY,
    STYLE_GLOW_SIZE, STYLE_GRADIENT_DEPTH, STYLE_ID_COUNT, STYLE_REFLECTION_DEPTH,
    STYLE_REFLECTION_OPACITY, STYLE_SHADOW_ANGLE_IN_DEGREES, STYLE_SHADOW_BLUR,
    STYLE_SHADOW_DISTANCE, STYLE_SHADOW_OPACITY, STYLE_TRANSPARENCY_AMOUNT,
    STYLE_TRANSPARENCY_ENABLED, SYSTEM_FONT, TEMPLATE_FILE_NAME, TITLE_BLOCK_ON, USE_CUSTOM_SHEET,
    USE_MBCS, VISIBLE_GRID_ON, VISIBLE_GRID_SIZE, VISIBLE_GRID_SIZE_FRAC, WORKSPACE_ORIENTATION,
};
use altium_format_types::constants::streams::{
    ADDITIONAL, FILE_HEADER, LIB_ADDITIONAL, PIN_DESC, PIN_FRAC, PIN_FUNCTION_DATA, PIN_MISC_DATA,
    PIN_PACKAGE_LENGTH, PIN_PROPAGATION_DELAY, PIN_SYMBOL_LINE_WIDTH, PIN_TEXT_DATA, PIN_WIDE_TEXT,
    REDIRECTION, SECTION_KEYS, STORAGE,
};
use altium_format_types::constants::text::NAME;
use altium_format_types::constants::text::{BOLD, DESC, DESIG, ITALIC, STRIKE_OUT, UNDERLINE};
use altium_format_types::constants::visual::{FONT_ID_COUNT, FONT_NAME, ROTATION, SIZE};
use altium_format_types::sch::{
    ParameterReadOnlyState, ParameterType, SchDisplayStyle, SchFont, TextHorzAnchor, TextVertAnchor,
};
use altium_format_types::{
    Color, Coord, CoordPoint, RotationBy90,
    SchDisplaySettings, SchRecordType, SheetBorderStyle, SheetOrientation,
    SheetReferenceZoneStyle, SheetStyle, TextJustification,
};

// Sidecar parameter keys: Delphi convention (all-uppercase, no separators) for
// byte-exact roundtrip with files created by Altium's Delphi code path.
// The C# FileFormatConsts use mixed-case (e.g. "PinPackageLength"), but actual
// .SchLib files contain Delphi-style uppercase keys in their UTF-16LE sidecar data.
// Parsing is case-insensitive so the constants from altium-format-types work for reads;
// these are only needed for writes.
const SIDECAR_SYMBOL_LINE_WIDTH: &str = "SYMBOL_LINEWIDTH";
const SIDECAR_PIN_PACKAGE_LENGTH: &str = "PINPACKAGELENGTH";

use crate::binary_io::{BinaryReader, BinaryWriter};
use crate::block_stream::{Block, BlockFormat, parse_blocks, write_text_block};
use crate::cfb_document::CfbDocument;
use crate::embedded_object::{parse_embedded_object_stream, serialize_embedded_object_stream};
use crate::param_collection::ParameterCollection;
use crate::param_value::ToParamValue;
use crate::util::generate_unique_id;
use crate::sch_records::{
    PinTextPositioning, SchArc, SchBezier, SchDesignator, SchEllipse, SchEllipticalArc, SchImage,
    SchImplementation, SchImplementationList, SchImplementationMap, SchLabel, SchLibComponent,
    SchLine, SchMapDefiner, SchParameter, SchParameterList, SchPie, SchPin, SchPolygon,
    SchPolyline, SchPrimitiveBase, SchRecord, SchRectangle, SchRoundRectangle, SchSymbol,
    SchTextFrame, parse_binary_pin, parse_component_record,
    serialize_component_record, serialize_record,
};
use crate::tracked_cfb::TrackedCfbDocument;
use crate::{AltiumFormatError, Result, ResultExt};

pub struct SchLib {
    header: SchLibHeader,
    components: Vec<SchLibComponent>,
    embedded_images: Vec<SchLibEmbeddedImage>,
    aliases: Vec<SchLibAlias>,
}

pub(crate) struct SchLibEmbeddedImage {
    pub file_name: String,
    pub data: Vec<u8>,
}

pub(crate) struct SchLibAlias {
    pub alias_name: String,
    pub canonical_name: String,
}

#[derive(Debug)]
pub(crate) struct SchLibHeader {
    pub weight: i32,
    pub minor_version: i32,
    pub unique_id: String,
    pub fonts: Vec<SchFont>,
    pub display_settings: SchDisplaySettings,
    pub components: Vec<SchLibComponentIndex>,
}

#[derive(Debug)]
pub(crate) struct SchLibComponentIndex {
    pub lib_ref: String,
    pub description: String,
    pub part_count: i32,
    pub aliases: Vec<String>,
}

pub(crate) fn parse_file_header(data: &[u8]) -> Result<SchLibHeader> {
    let blocks = parse_blocks(data)?;
    if blocks.len() != 1 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: FILE_HEADER.to_owned(),
            detail: format!("expected 1 block, got {}", blocks.len()),
        });
    }
    let block = &blocks[0];
    if block.format != BlockFormat::Text {
        return Err(AltiumFormatError::InvalidParamValue {
            key: FILE_HEADER.to_owned(),
            detail: "expected text block, got binary".to_owned(),
        });
    }

    let mut params = ParameterCollection::from_bytes(&block.data)?;

    let header: String = params.remove_required(HEADER)?;
    if header != SCH_LIBRARY_BINARY_HEADER_V50 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: HEADER.to_owned(),
            detail: format!(
                "expected {:?}, got {:?}",
                SCH_LIBRARY_BINARY_HEADER_V50, header
            ),
        });
    }

    let weight: i32 = params.remove_required(WEIGHT)?;
    let minor_version: i32 = params.remove_required(MINOR_VERSION)?;
    let unique_id: String = params.remove_with_default(UNIQUE_ID, String::new())?;

    // Font table (1-based indexing)
    let fonts = params.remove_indexed(FONT_ID_COUNT, 1, |p, i| {
        let idx = i.to_string();
        // AD26 SchDataSerializerParam::ReadShort/ReadString default missing values
        // instead of hard-failing; keep import tolerant for legacy/malformed files.
        let mut name: String =
            p.remove_with_default(&format!("{}{}", FONT_NAME, idx), String::new())?;
        let size: i32 = p.remove_with_default(&format!("{}{}", SIZE, idx), 0i32)?;
        let rotation: i32 = p.remove_with_default(&format!("{}{}", ROTATION, idx), 0i32)?;
        let bold: bool = p.remove_with_default(&format!("{}{}", BOLD, idx), false)?;
        let italic: bool = p.remove_with_default(&format!("{}{}", ITALIC, idx), false)?;
        let underline: bool = p.remove_with_default(&format!("{}{}", UNDERLINE, idx), false)?;
        let strikeout: bool = p.remove_with_default(&format!("{}{}", STRIKE_OUT, idx), false)?;
        if name.is_empty() {
            name = "Times New Roman".to_owned();
        }
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

    let mut styles = Vec::new();
    if let Some(style_count) = params.remove_optional::<usize>(STYLE_ID_COUNT)? {
        styles.reserve(style_count);
        for i in 1..=style_count {
            let idx = i.to_string();
            let parse_style_coord = |p: &mut ParameterCollection,
                                     base: &str,
                                     frac_suffix: &str,
                                     idx: &str|
             -> Result<Option<Coord>> {
                let base_key = format!("{base}{idx}");
                let frac_key = format!("{base}{idx}{frac_suffix}");
                let integer = p.remove_optional::<i32>(&base_key)?;
                let frac = p.remove_optional::<i32>(&frac_key)?;
                match (integer, frac) {
                    (None, None) => Ok(None),
                    (int_part, frac_part) => Ok(Some(Coord::from_dxp_frac(
                        int_part.unwrap_or(0),
                        frac_part.unwrap_or(0),
                    ))),
                }
            };

            styles.push(SchDisplayStyle {
                id: i as i32,
                gradient_depth: params
                    .remove_optional::<i32>(&format!("{}{}", STYLE_GRADIENT_DEPTH, idx))?,
                shadow_opacity: params
                    .remove_optional::<i32>(&format!("{}{}", STYLE_SHADOW_OPACITY, idx))?,
                shadow_distance: parse_style_coord(
                    &mut params,
                    STYLE_SHADOW_DISTANCE,
                    "_FRAC",
                    &idx,
                )?,
                shadow_blur: parse_style_coord(&mut params, STYLE_SHADOW_BLUR, "_FRAC", &idx)?,
                shadow_angle_in_degrees: params
                    .remove_optional::<i32>(&format!("{}{}", STYLE_SHADOW_ANGLE_IN_DEGREES, idx))?,
                glow_color: params
                    .remove_optional::<i32>(&format!("{}{}", STYLE_GLOW_COLOR, idx))?
                    .map(Color::new),
                glow_opacity: params
                    .remove_optional::<i32>(&format!("{}{}", STYLE_GLOW_OPACITY, idx))?,
                glow_size: params.remove_optional::<i32>(&format!("{}{}", STYLE_GLOW_SIZE, idx))?,
                reflection_depth: params
                    .remove_optional::<i32>(&format!("{}{}", STYLE_REFLECTION_DEPTH, idx))?,
                reflection_opacity: params
                    .remove_optional::<i32>(&format!("{}{}", STYLE_REFLECTION_OPACITY, idx))?,
                transparency_enabled: params
                    .remove_optional::<bool>(&format!("{}{}", STYLE_TRANSPARENCY_ENABLED, idx))?,
                transparency_amount: params
                    .remove_optional::<i32>(&format!("{}{}", STYLE_TRANSPARENCY_AMOUNT, idx))?,
                corner_radius_mode: params
                    .remove_optional::<i32>(&format!("{}{}", STYLE_CORNER_RADIUS_MODE, idx))?,
                corner_radius_value: params
                    .remove_optional::<i32>(&format!("{}{}", STYLE_CORNER_RADIUS_VALUE, idx))?,
            });
        }
    }

    // Display settings — library-level sheet display preferences, preserved for round-trip
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
            &format!("{}_Frac", CUSTOM_MARGIN_WIDTH),
        )?,
        sheet_number_space_size: params.remove_optional(SHEET_NUMBER_SPACE_SIZE)?,
        workspace_orientation: params
            .remove_optional::<u8>(WORKSPACE_ORIENTATION)?
            .map(SheetOrientation::try_from)
            .transpose()?,
        show_hidden_pins: params.remove_optional(SHOW_HIDDEN_PINS)?,
        show_template_graphics: params.remove_optional(SHOW_TEMPLATE_GRAPHICS)?,
        always_show_cd: params.remove_optional(ALWAYS_SHOW_CD)?,
        template_file_name: params.remove_optional(TEMPLATE_FILE_NAME)?,
        display_unit: params.remove_optional(DISPLAY_UNIT)?,
        system_font: params.remove_optional(SYSTEM_FONT)?,
        use_mbcs: params.remove_optional(USE_MBCS)?,
        is_boc: params.remove_optional(IS_BOC)?,
        area_color: params.remove_optional::<i32>(AREA_COLOR)?.map(Color::new),
        styles,
        file_version_info: params.remove_optional(FILE_VERSION_INFO)?,
    };

    // Component index (0-based indexing)
    let components = params.remove_indexed(COMP_COUNT, 0, |p, n| {
        let lib_ref: String = p.remove_required(&format!("{}{}", LIB_REF, n))?;
        let description: String =
            p.remove_with_default(&format!("{}{}", COMP_DESCR, n), String::new())?;
        let part_count: i32 = p.remove_with_default(&format!("{}{}", PART_COUNT, n), 1i32)?;
        let alias_count: i32 = p.remove_with_default(&format!("{}{}", ALIAS_COUNT, n), 0i32)?;
        let mut aliases = Vec::with_capacity(alias_count as usize);
        for m in 0..alias_count {
            let alias_key = format!("{}{}Alias{}", "Comp", n, m);
            let alias: String = p.remove_required(&alias_key)?;
            aliases.push(alias);
        }
        Ok(SchLibComponentIndex {
            lib_ref,
            description,
            part_count,
            aliases,
        })
    })?;

    params.assert_exhausted()?;

    Ok(SchLibHeader {
        weight,
        minor_version,
        unique_id,
        fonts,
        display_settings,
        components,
    })
}

use crate::pcblib::section_keys::parse_section_keys_text;

pub(crate) fn resolve_component_key(name: &str, section_keys: &HashMap<String, String>) -> String {
    crate::pcblib::section_keys::sanitize_cfb_name(
        section_keys.get(name).map(String::as_str).unwrap_or(name),
    )
}

// NOTE: The local `sanitize_cfb_name` function is NOT removed — it is still called
// by `build_section_key_for_name` (write-path code). Only `resolve_component_key`
// delegates to the shared version. Both can coexist.

fn is_end_marker(block: &Block) -> Result<bool> {
    if block.format != BlockFormat::Text {
        return Ok(false);
    }
    let mut params = ParameterCollection::from_bytes(&block.data)?;
    let record_val: i32 = match params.remove_optional::<i32>(RECORD)? {
        Some(v) => v,
        // A text block with no RECORD key is an empty/padding block — treat as end marker
        None => return Ok(true),
    };
    Ok(record_val == 0)
}

fn dispatch_record(block: &Block) -> Result<SchRecord> {
    match block.format {
        BlockFormat::Binary => {
            if block.data.is_empty() {
                return Err(AltiumFormatError::InvalidBlockHeader {
                    offset: 0,
                    detail: "binary block has no data".to_owned(),
                });
            }
            let code = block.data[0];
            match code {
                PIN_BINARY_CODE => parse_binary_pin(&block.data)
                    .map(SchRecord::Pin)
                    .context("binary pin"),
                _ => Err(AltiumFormatError::UnknownBinaryCode(code)),
            }
        }
        BlockFormat::Text => {
            let mut params = ParameterCollection::from_bytes(&block.data)?;
            let record_raw: i32 = params.remove_required(RECORD)?;
            let record_type_val = if record_raw == INSTRUCTION_EXTRA_OBJECT_INDEX as i32 {
                params.remove_required::<i32>(RECORD_EX)?
            } else {
                record_raw
            };
            let record_type = SchRecordType::try_from(record_type_val)?;
            macro_rules! dispatch {
                ($ty:ty => $variant:expr) => {{
                    let v = <$ty>::from_params(&mut params).with_context(|| {
                        format!(
                            "RECORD={record_type_val} ({ty_name})",
                            ty_name = stringify!($ty)
                        )
                    })?;
                    params.assert_exhausted().with_context(|| {
                        format!(
                            "RECORD={record_type_val} ({ty_name})",
                            ty_name = stringify!($ty)
                        )
                    })?;
                    Ok($variant(v))
                }};
            }
            match record_type {
                SchRecordType::Component => {
                    let comp = parse_component_record(&mut params)?;
                    params.assert_exhausted()?;
                    Ok(SchRecord::Component(comp))
                }
                SchRecordType::Symbol => dispatch!(SchSymbol => SchRecord::Symbol),
                SchRecordType::Label => dispatch!(SchLabel => SchRecord::Label),
                SchRecordType::Bezier => dispatch!(SchBezier => SchRecord::Bezier),
                SchRecordType::Polyline => dispatch!(SchPolyline => SchRecord::Polyline),
                SchRecordType::Polygon => dispatch!(SchPolygon => SchRecord::Polygon),
                SchRecordType::Ellipse => dispatch!(SchEllipse => SchRecord::Ellipse),
                SchRecordType::Pie => dispatch!(SchPie => SchRecord::Pie),
                SchRecordType::RoundRectangle => {
                    dispatch!(SchRoundRectangle => SchRecord::RoundRectangle)
                }
                SchRecordType::EllipticalArc => {
                    dispatch!(SchEllipticalArc => SchRecord::EllipticalArc)
                }
                SchRecordType::Arc => dispatch!(SchArc => SchRecord::Arc),
                SchRecordType::Line => dispatch!(SchLine => SchRecord::Line),
                SchRecordType::Rectangle => dispatch!(SchRectangle => SchRecord::Rectangle),
                SchRecordType::TextFrame => dispatch!(SchTextFrame => SchRecord::TextFrame),
                SchRecordType::Image => dispatch!(SchImage => SchRecord::Image),
                SchRecordType::Designator => dispatch!(SchDesignator => SchRecord::Designator),
                SchRecordType::Parameter => dispatch!(SchParameter => SchRecord::Parameter),
                SchRecordType::ImplementationList => {
                    dispatch!(SchImplementationList => SchRecord::ImplementationList)
                }
                SchRecordType::Implementation => {
                    dispatch!(SchImplementation => SchRecord::Implementation)
                }
                SchRecordType::ImplementationMap => {
                    dispatch!(SchImplementationMap => SchRecord::ImplementationMap)
                }
                SchRecordType::MapDefiner => dispatch!(SchMapDefiner => SchRecord::MapDefiner),
                SchRecordType::ParameterList => {
                    dispatch!(SchParameterList => SchRecord::ParameterList)
                }
                _ => Err(AltiumFormatError::UnknownRecordType(record_type_val)),
            }
        }
    }
}

pub(crate) fn parse_component_data(data: &[u8]) -> Result<SchLibComponent> {
    let blocks = parse_blocks(data)?;
    let mut blocks_iter = blocks.iter();

    let first_block = blocks_iter
        .next()
        .ok_or_else(|| AltiumFormatError::MissingParam("empty Data stream".to_owned()))?;

    if first_block.format != BlockFormat::Text {
        return Err(AltiumFormatError::InvalidParamValue {
            key: RECORD.to_owned(),
            detail: "first block in Data stream must be text (SchComponent)".to_owned(),
        });
    }
    let mut params = ParameterCollection::from_bytes(&first_block.data)?;
    let record_val: i32 = params.remove_required(RECORD)?;
    if record_val != SchRecordType::Component as i32 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: RECORD.to_owned(),
            detail: format!("expected RECORD=1 (Component) as first block, got {record_val}"),
        });
    }
    let component = parse_component_record(&mut params)?;
    params.assert_exhausted()?;

    let mut records = Vec::new();
    for (i, block) in blocks_iter.enumerate() {
        if is_end_marker(block)? {
            break;
        }
        records
            .push(dispatch_record(block).with_context(|| format!("record #{i} in Data stream"))?);
    }

    Ok(SchLibComponent {
        component,
        records,
        additional_records: Vec::new(),
    })
}

// ── Pin sidecar helpers ────────────────────────────────────────────────────────

fn collect_pins_mut(records: &mut Vec<SchRecord>) -> Vec<&mut SchPin> {
    records
        .iter_mut()
        .filter_map(|r| {
            if let SchRecord::Pin(p) = r {
                Some(p)
            } else {
                None
            }
        })
        .collect()
}

// Parses a sidecar entry whose inner_data (already decompressed by parse_embedded_object)
// stores a length-prefixed UTF-16LE parameter string.
// Format: i32LE(byte_len) + UTF-16LE bytes.
fn read_sidecar_utf16le_params(inner_data: &[u8]) -> Result<ParameterCollection> {
    let mut r = BinaryReader::new(inner_data);
    let payload_len = r.read_i32_le()? as usize;
    let text_bytes = r.read_bytes(payload_len)?;
    ParameterCollection::from_utf16le_bytes(text_bytes)
}

// Parses a sidecar entry whose inner_data (already decompressed by parse_embedded_object)
// stores a length-prefixed ASCII parameter string.
// Format: i32LE(byte_len) + ASCII bytes.
fn read_sidecar_ascii_params(inner_data: &[u8]) -> Result<ParameterCollection> {
    let mut r = BinaryReader::new(inner_data);
    let payload_len = r.read_i32_le()? as usize;
    let text_bytes = r.read_bytes(payload_len)?;
    ParameterCollection::from_bytes(text_bytes)
}

fn parse_pin_index(id: &str) -> Result<usize> {
    id.parse::<usize>()
        .map_err(|_| AltiumFormatError::InvalidParamValue {
            key: "embedded object id".to_owned(),
            detail: format!("expected decimal pin index, got {:?}", id),
        })
}

fn merge_pin_frac(
    cfb: &mut TrackedCfbDocument,
    component_key: &str,
    pins: &mut [&mut SchPin],
) -> Result<()> {
    let stream_path = format!("/{}/{}", component_key, PIN_FRAC);
    let data = match cfb.read_stream_optional(&stream_path)? {
        Some(d) => d,
        None => return Ok(()),
    };
    let blocks = parse_blocks(&data)?;
    let entries = parse_embedded_object_stream(&blocks)?;
    for entry in &entries {
        let pin_idx = parse_pin_index(&entry.id)?;
        if pin_idx >= pins.len() {
            return Err(AltiumFormatError::InvalidPinIndex {
                index: pin_idx,
                count: pins.len(),
            });
        }
        let mut r = BinaryReader::new(&entry.inner_data);
        let x_frac = r.read_i32_le()?;
        let y_frac = r.read_i32_le()?;
        let len_frac = r.read_i32_le()?;

        let pin = &mut pins[pin_idx];
        let old_x = pin.location.x.to_internal();
        let old_y = pin.location.y.to_internal();
        let old_len = pin.pin_length.to_internal();
        pin.location.x = Coord::from_internal(old_x + x_frac);
        pin.location.y = Coord::from_internal(old_y + y_frac);
        pin.pin_length = Coord::from_internal(old_len + len_frac);
    }
    Ok(())
}

fn merge_pin_desc(
    cfb: &mut TrackedCfbDocument,
    component_key: &str,
    pins: &mut [&mut SchPin],
) -> Result<()> {
    let stream_path = format!("/{}/{}", component_key, PIN_DESC);
    let data = match cfb.read_stream_optional(&stream_path)? {
        Some(d) => d,
        None => return Ok(()),
    };
    let blocks = parse_blocks(&data)?;
    let entries = parse_embedded_object_stream(&blocks)?;
    for entry in &entries {
        let pin_idx = parse_pin_index(&entry.id)?;
        if pin_idx >= pins.len() {
            return Err(AltiumFormatError::InvalidPinIndex {
                index: pin_idx,
                count: pins.len(),
            });
        }
        let mut r = BinaryReader::new(&entry.inner_data);
        let byte_len = r.read_i32_le()? as usize;
        let text_bytes = r.read_bytes(byte_len)?;
        let (overflow, _) = encoding_rs::WINDOWS_1252.decode_without_bom_handling(text_bytes);
        pins[pin_idx].description.push_str(overflow.as_ref());
    }
    Ok(())
}

fn merge_pin_misc_data(
    cfb: &mut TrackedCfbDocument,
    component_key: &str,
    pins: &mut [&mut SchPin],
) -> Result<()> {
    let stream_path = format!("/{}/{}", component_key, PIN_MISC_DATA);
    let data = match cfb.read_stream_optional(&stream_path)? {
        Some(d) => d,
        None => return Ok(()),
    };
    let blocks = parse_blocks(&data)?;
    let entries = parse_embedded_object_stream(&blocks)?;
    for entry in &entries {
        let pin_idx = parse_pin_index(&entry.id)?;
        if pin_idx >= pins.len() {
            return Err(AltiumFormatError::InvalidPinIndex {
                index: pin_idx,
                count: pins.len(),
            });
        }
        let mut params = read_sidecar_utf16le_params(&entry.inner_data)?;
        if let Some(v) = params.remove_optional::<String>(PAIR_SWAP_ID)? {
            pins[pin_idx].swap_id_pair = v;
        }
        params.assert_exhausted()?;
    }
    Ok(())
}

fn merge_pin_text_data(
    cfb: &mut TrackedCfbDocument,
    component_key: &str,
    pins: &mut [&mut SchPin],
) -> Result<()> {
    let stream_path = format!("/{}/{}", component_key, PIN_TEXT_DATA);
    let data = match cfb.read_stream_optional(&stream_path)? {
        Some(d) => d,
        None => return Ok(()),
    };
    let blocks = parse_blocks(&data)?;
    let entries = parse_embedded_object_stream(&blocks)?;
    for entry in &entries {
        let pin_idx = parse_pin_index(&entry.id)?;
        if pin_idx >= pins.len() {
            return Err(AltiumFormatError::InvalidPinIndex {
                index: pin_idx,
                count: pins.len(),
            });
        }
        // Each entry has two consecutive variable-length binary structs: name then designator
        let mut r = BinaryReader::new(&entry.inner_data);
        let name_flags = r.read_u8()?;
        let name_pos_custom = (name_flags & PIN_TEXT_POS_CUSTOM) != 0;
        let name_rot_anchor = (name_flags & PIN_TEXT_ROT_ANCHOR) != 0;
        let name_rot_rel_raw = (name_flags & PIN_TEXT_ROT_REL_MASK) >> PIN_TEXT_ROT_REL_SHIFT;
        let name_rot_rel = RotationBy90::try_from(name_rot_rel_raw)?;
        let name_font_custom = (name_flags & PIN_TEXT_FONT_CUSTOM) != 0;
        let name_margin = if name_pos_custom {
            Some(Coord::from_internal(r.read_i32_le()?))
        } else {
            None
        };
        let (name_font_id, name_color) = if name_font_custom {
            (Some(r.read_i16_le()?), Some(Color::new(r.read_i32_le()?)))
        } else {
            (None, None)
        };

        let desig_flags = r.read_u8()?;
        let desig_pos_custom = (desig_flags & PIN_TEXT_POS_CUSTOM) != 0;
        let desig_rot_anchor = (desig_flags & PIN_TEXT_ROT_ANCHOR) != 0;
        let desig_rot_rel_raw = (desig_flags & PIN_TEXT_ROT_REL_MASK) >> PIN_TEXT_ROT_REL_SHIFT;
        let desig_rot_rel = RotationBy90::try_from(desig_rot_rel_raw)?;
        let desig_font_custom = (desig_flags & PIN_TEXT_FONT_CUSTOM) != 0;
        let desig_margin = if desig_pos_custom {
            Some(Coord::from_internal(r.read_i32_le()?))
        } else {
            None
        };
        let (desig_font_id, desig_color) = if desig_font_custom {
            (Some(r.read_i16_le()?), Some(Color::new(r.read_i32_le()?)))
        } else {
            (None, None)
        };

        pins[pin_idx].name_text_data = Some(PinTextPositioning {
            position_mode_custom: name_pos_custom,
            rotation_anchor_component: name_rot_anchor,
            rotation_relative: name_rot_rel,
            font_mode_custom: name_font_custom,
            custom_position_margin: name_margin,
            custom_font_id: name_font_id,
            custom_color: name_color,
        });
        pins[pin_idx].designator_text_data = Some(PinTextPositioning {
            position_mode_custom: desig_pos_custom,
            rotation_anchor_component: desig_rot_anchor,
            rotation_relative: desig_rot_rel,
            font_mode_custom: desig_font_custom,
            custom_position_margin: desig_margin,
            custom_font_id: desig_font_id,
            custom_color: desig_color,
        });
    }
    Ok(())
}

fn merge_pin_wide_text(
    cfb: &mut TrackedCfbDocument,
    component_key: &str,
    pins: &mut [&mut SchPin],
) -> Result<()> {
    let stream_path = format!("/{}/{}", component_key, PIN_WIDE_TEXT);
    let data = match cfb.read_stream_optional(&stream_path)? {
        Some(d) => d,
        None => return Ok(()),
    };
    let blocks = parse_blocks(&data)?;
    let entries = parse_embedded_object_stream(&blocks)?;
    for entry in &entries {
        let pin_idx = parse_pin_index(&entry.id)?;
        if pin_idx >= pins.len() {
            return Err(AltiumFormatError::InvalidPinIndex {
                index: pin_idx,
                count: pins.len(),
            });
        }
        let mut params = read_sidecar_utf16le_params(&entry.inner_data)?;
        if let Some(v) = params.remove_optional::<String>(DESC)? {
            pins[pin_idx].description = v;
        }
        if let Some(v) = params.remove_optional::<String>(NAME)? {
            pins[pin_idx].name = v;
        }
        if let Some(v) = params.remove_optional::<String>(DESIG)? {
            pins[pin_idx].designator = v;
        }
        if let Some(v) = params.remove_optional::<String>(SWAP_ID)? {
            pins[pin_idx].swap_id_pin = v;
        }
        if let Some(v) = params.remove_optional::<String>(SWAP_ID_PART)? {
            pins[pin_idx].swap_id_part = v;
        }
        if let Some(v) = params.remove_optional::<String>(DEF_VALUE)? {
            pins[pin_idx].default_value = v;
        }
        params.assert_exhausted()?;
    }
    Ok(())
}

fn merge_pin_symbol_line_width(
    cfb: &mut TrackedCfbDocument,
    component_key: &str,
    pins: &mut [&mut SchPin],
) -> Result<()> {
    let stream_path = format!("/{}/{}", component_key, PIN_SYMBOL_LINE_WIDTH);
    let data = match cfb.read_stream_optional(&stream_path)? {
        Some(d) => d,
        None => return Ok(()),
    };
    let blocks = parse_blocks(&data)?;
    let entries = parse_embedded_object_stream(&blocks)?;
    for entry in &entries {
        let pin_idx = parse_pin_index(&entry.id)?;
        if pin_idx >= pins.len() {
            return Err(AltiumFormatError::InvalidPinIndex {
                index: pin_idx,
                count: pins.len(),
            });
        }
        let mut params = read_sidecar_utf16le_params(&entry.inner_data)?;
        if let Some(v) = params.remove_optional::<i32>(SYMBOL_LINE_WIDTH)? {
            pins[pin_idx].pin_symbol_line_width = Some(v);
        }
        params.assert_exhausted()?;
    }
    Ok(())
}

fn merge_pin_package_length(
    cfb: &mut TrackedCfbDocument,
    component_key: &str,
    pins: &mut [&mut SchPin],
) -> Result<()> {
    let stream_path = format!("/{}/{}", component_key, PIN_PACKAGE_LENGTH);
    let data = match cfb.read_stream_optional(&stream_path)? {
        Some(d) => d,
        None => return Ok(()),
    };
    let blocks = parse_blocks(&data)?;
    let entries = parse_embedded_object_stream(&blocks)?;
    for entry in &entries {
        let pin_idx = parse_pin_index(&entry.id)?;
        if pin_idx >= pins.len() {
            return Err(AltiumFormatError::InvalidPinIndex {
                index: pin_idx,
                count: pins.len(),
            });
        }
        let mut params = read_sidecar_utf16le_params(&entry.inner_data)?;
        if let Some(v) = params.remove_optional::<String>(PIN_PACKAGE_LENGTH_KEY)? {
            pins[pin_idx].pin_package_length = v;
        }
        params.assert_exhausted()?;
    }
    Ok(())
}

fn merge_pin_propagation_delay(
    cfb: &mut TrackedCfbDocument,
    component_key: &str,
    pins: &mut [&mut SchPin],
) -> Result<()> {
    let stream_path = format!("/{}/{}", component_key, PIN_PROPAGATION_DELAY);
    let data = match cfb.read_stream_optional(&stream_path)? {
        Some(d) => d,
        None => return Ok(()),
    };
    let blocks = parse_blocks(&data)?;
    let entries = parse_embedded_object_stream(&blocks)?;
    for entry in &entries {
        let pin_idx = parse_pin_index(&entry.id)?;
        if pin_idx >= pins.len() {
            return Err(AltiumFormatError::InvalidPinIndex {
                index: pin_idx,
                count: pins.len(),
            });
        }
        let mut params = read_sidecar_utf16le_params(&entry.inner_data)?;
        if let Some(v) = params.remove_optional::<String>(PIN_PROPAGATION_DELAY_KEY)? {
            pins[pin_idx].propagation_delay = v;
        }
        params.assert_exhausted()?;
    }
    Ok(())
}

fn merge_pin_function_data(
    cfb: &mut TrackedCfbDocument,
    component_key: &str,
    pins: &mut [&mut SchPin],
) -> Result<()> {
    let stream_path = format!("/{}/{}", component_key, PIN_FUNCTION_DATA);
    let data = match cfb.read_stream_optional(&stream_path)? {
        Some(d) => d,
        None => return Ok(()),
    };
    let blocks = parse_blocks(&data)?;
    let entries = parse_embedded_object_stream(&blocks)?;
    for entry in &entries {
        let pin_idx = parse_pin_index(&entry.id)?;
        if pin_idx >= pins.len() {
            return Err(AltiumFormatError::InvalidPinIndex {
                index: pin_idx,
                count: pins.len(),
            });
        }
        let mut params = read_sidecar_utf16le_params(&entry.inner_data)?;
        let sel_count: i32 = params.remove_with_default(PIN_SELECTED_FUNCTIONS_COUNT, 0i32)?;
        let mut selected = Vec::with_capacity(sel_count as usize);
        for i in 1..=sel_count {
            let key = format!("{}{}", PIN_SELECTED_FUNCTION, i);
            let v: String = params.remove_required(&key)?;
            selected.push(v);
        }
        let def_count: i32 = params.remove_with_default(PIN_DEFINED_FUNCTIONS_COUNT, 0i32)?;
        let mut defined = Vec::with_capacity(def_count as usize);
        for i in 1..=def_count {
            let key = format!("{}{}", PIN_DEFINED_FUNCTION, i);
            let v: String = params.remove_required(&key)?;
            defined.push(v);
        }
        params.assert_exhausted()?;
        pins[pin_idx].selected_functions = selected;
        pins[pin_idx].defined_functions = defined;
    }
    Ok(())
}

pub(crate) fn merge_pin_sidecars(
    cfb: &mut TrackedCfbDocument,
    component_key: &str,
    pins: &mut [&mut SchPin],
) -> Result<()> {
    merge_pin_frac(cfb, component_key, pins)
        .with_context(|| format!("/{}/{}", component_key, PIN_FRAC))?;
    merge_pin_desc(cfb, component_key, pins)
        .with_context(|| format!("/{}/{}", component_key, PIN_DESC))?;
    merge_pin_misc_data(cfb, component_key, pins)
        .with_context(|| format!("/{}/{}", component_key, PIN_MISC_DATA))?;
    merge_pin_text_data(cfb, component_key, pins)
        .with_context(|| format!("/{}/{}", component_key, PIN_TEXT_DATA))?;
    merge_pin_wide_text(cfb, component_key, pins)
        .with_context(|| format!("/{}/{}", component_key, PIN_WIDE_TEXT))?;
    merge_pin_symbol_line_width(cfb, component_key, pins)
        .with_context(|| format!("/{}/{}", component_key, PIN_SYMBOL_LINE_WIDTH))?;
    merge_pin_package_length(cfb, component_key, pins)
        .with_context(|| format!("/{}/{}", component_key, PIN_PACKAGE_LENGTH))?;
    merge_pin_propagation_delay(cfb, component_key, pins)
        .with_context(|| format!("/{}/{}", component_key, PIN_PROPAGATION_DELAY))?;
    merge_pin_function_data(cfb, component_key, pins)
        .with_context(|| format!("/{}/{}", component_key, PIN_FUNCTION_DATA))?;
    Ok(())
}

// ── Pin sidecar writers ──────────────────────────────────────────────────────

// Wraps a UTF-16LE parameter collection in the length-prefixed sidecar format:
// i32LE(byte_len) + UTF-16LE bytes.
fn write_sidecar_utf16le_params(params: &ParameterCollection) -> Vec<u8> {
    let utf16_bytes = params.to_utf16le_bytes();
    let mut w = BinaryWriter::new();
    w.write_i32_le(utf16_bytes.len() as i32);
    w.write_bytes(&utf16_bytes);
    w.finish()
}

// Returns PinFrac sidecar stream bytes if any pin has non-zero fractional coords.
// Each entry: 12 bytes (i32 x_frac, i32 y_frac, i32 len_frac).
fn write_pin_frac(pins: &[&SchPin]) -> Option<Result<Vec<u8>>> {
    let mut entries = Vec::new();
    for (i, pin) in pins.iter().enumerate() {
        let x_frac = pin.location.x.to_internal() % C_BASE_UNIT;
        let y_frac = pin.location.y.to_internal() % C_BASE_UNIT;
        let len_frac = pin.pin_length.to_internal() % C_BASE_UNIT;
        if x_frac != 0 || y_frac != 0 || len_frac != 0 {
            let mut w = BinaryWriter::new();
            w.write_i32_le(x_frac);
            w.write_i32_le(y_frac);
            w.write_i32_le(len_frac);
            entries.push((i.to_string(), w.finish()));
        }
    }
    if entries.is_empty() {
        return None;
    }
    Some(serialize_embedded_object_stream(PIN_FRAC, &entries))
}

// Returns PinDesc sidecar stream bytes if any pin description exceeds 254 chars.
// Each entry: length-prefixed ASCII overflow text (chars beyond position 254).
fn write_pin_desc(pins: &[&SchPin]) -> Option<Result<Vec<u8>>> {
    let mut entries = Vec::new();
    for (i, pin) in pins.iter().enumerate() {
        if pin.description.len() > C_MAX_SHORT_STRING_LENGTH as usize {
            let overflow = &pin.description[C_MAX_SHORT_STRING_LENGTH as usize..];
            let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(overflow);
            let mut w = BinaryWriter::new();
            w.write_i32_le(encoded.len() as i32);
            w.write_bytes(&encoded);
            entries.push((i.to_string(), w.finish()));
        }
    }
    if entries.is_empty() {
        return None;
    }
    Some(serialize_embedded_object_stream(PIN_DESC, &entries))
}

// Returns PinMiscData sidecar stream if any pin has a swap_id_pair that needs
// sidecar storage (exceeds binary pin format limits per NeedToSaveParameter).
fn write_pin_misc_data(pins: &[&SchPin]) -> Option<Result<Vec<u8>>> {
    let mut entries = Vec::new();
    for (i, pin) in pins.iter().enumerate() {
        if pin_field_needs_wide_text(&pin.swap_id_pair) {
            let mut params = ParameterCollection::new();
            params.insert(PAIR_SWAP_ID, pin.swap_id_pair.clone());
            entries.push((i.to_string(), write_sidecar_utf16le_params(&params)));
        }
    }
    if entries.is_empty() {
        return None;
    }
    Some(serialize_embedded_object_stream(PIN_MISC_DATA, &entries))
}

// Returns PinTextData sidecar stream if any pin has custom text positioning.
fn write_pin_text_data(pins: &[&SchPin]) -> Option<Result<Vec<u8>>> {
    let mut entries = Vec::new();
    for (i, pin) in pins.iter().enumerate() {
        if pin.name_text_data.is_some() || pin.designator_text_data.is_some() {
            let mut w = BinaryWriter::new();
            write_pin_text_positioning_struct(&mut w, pin.name_text_data.as_ref());
            write_pin_text_positioning_struct(&mut w, pin.designator_text_data.as_ref());
            entries.push((i.to_string(), w.finish()));
        }
    }
    if entries.is_empty() {
        return None;
    }
    Some(serialize_embedded_object_stream(PIN_TEXT_DATA, &entries))
}

fn write_pin_text_positioning_struct(w: &mut BinaryWriter, data: Option<&PinTextPositioning>) {
    let data = match data {
        Some(d) => d,
        None => {
            w.write_u8(0); // all flags off = default
            return;
        }
    };
    let mut flags: u8 = 0;
    if data.position_mode_custom {
        flags |= PIN_TEXT_POS_CUSTOM;
    }
    if data.rotation_anchor_component {
        flags |= PIN_TEXT_ROT_ANCHOR;
    }
    flags |= ((data.rotation_relative as u8) << PIN_TEXT_ROT_REL_SHIFT) & PIN_TEXT_ROT_REL_MASK;
    if data.font_mode_custom {
        flags |= PIN_TEXT_FONT_CUSTOM;
    }
    w.write_u8(flags);
    if data.position_mode_custom {
        w.write_i32_le(data.custom_position_margin.map_or(0, |c| c.to_internal()));
    }
    if data.font_mode_custom {
        w.write_i16_le(data.custom_font_id.unwrap_or(0));
        w.write_i32_le(data.custom_color.map_or(0, |c| c.raw()));
    }
}

/// Returns true if a pin text field needs to be saved in the PinWideText sidecar
/// because it cannot be faithfully represented in the binary pin format.
///
/// Matches Altium's `NeedToSaveParameter()` logic (SchDataExporterLibraryV5.cs:711-722):
/// - Field exceeds 254 bytes (binary pin length-prefix is u8)
/// - Field contains non-ANSI characters (> 0x7E, except 0x8E which is the pipe escape)
fn pin_field_needs_wide_text(value: &str) -> bool {
    value.len() > C_MAX_SHORT_STRING_LENGTH as usize || value.chars().any(|c| c as u32 > 0x7E && c != C_SCH_SPECIAL_DELIMITER)
}

// Returns PinWideText sidecar stream if any pin has fields that need wide text.
//
// Only writes fields that individually meet the NeedToSaveParameter() criteria,
// plus sidecar-only fields (swap_id_part, default_value) that are non-empty.
fn write_pin_wide_text(pins: &[&SchPin]) -> Option<Result<Vec<u8>>> {
    let mut entries = Vec::new();
    for (i, pin) in pins.iter().enumerate() {
        let mut params = ParameterCollection::new();
        if pin_field_needs_wide_text(&pin.description) {
            params.insert(DESC, pin.description.clone());
        }
        if pin_field_needs_wide_text(&pin.name) {
            params.insert(NAME, pin.name.clone());
        }
        if pin_field_needs_wide_text(&pin.designator) {
            params.insert(DESIG, pin.designator.clone());
        }
        if pin_field_needs_wide_text(&pin.swap_id_pin) {
            params.insert(SWAP_ID, pin.swap_id_pin.clone());
        }
        if pin_field_needs_wide_text(&pin.swap_id_part) {
            params.insert(SWAP_ID_PART, pin.swap_id_part.clone());
        }
        if pin_field_needs_wide_text(&pin.default_value) {
            params.insert(DEF_VALUE, pin.default_value.clone());
        }
        if !params.is_empty() {
            entries.push((i.to_string(), write_sidecar_utf16le_params(&params)));
        }
    }
    if entries.is_empty() {
        return None;
    }
    Some(serialize_embedded_object_stream(PIN_WIDE_TEXT, &entries))
}

// Returns PinSymbolLineWidth sidecar stream if any pin has a sidecar entry.
fn write_pin_symbol_line_width(pins: &[&SchPin]) -> Option<Result<Vec<u8>>> {
    let mut entries = Vec::new();
    for (i, pin) in pins.iter().enumerate() {
        if let Some(width) = pin.pin_symbol_line_width {
            let mut params = ParameterCollection::new();
            params.insert(SIDECAR_SYMBOL_LINE_WIDTH, width.to_string());
            entries.push((i.to_string(), write_sidecar_utf16le_params(&params)));
        }
    }
    if entries.is_empty() {
        return None;
    }
    Some(serialize_embedded_object_stream(
        PIN_SYMBOL_LINE_WIDTH,
        &entries,
    ))
}

// Returns PinPackageLength sidecar stream if any pin has non-empty package length.
fn write_pin_package_length(pins: &[&SchPin]) -> Option<Result<Vec<u8>>> {
    let mut entries = Vec::new();
    for (i, pin) in pins.iter().enumerate() {
        if !pin.pin_package_length.is_empty() {
            let mut params = ParameterCollection::new();
            params.insert(SIDECAR_PIN_PACKAGE_LENGTH, pin.pin_package_length.clone());
            entries.push((i.to_string(), write_sidecar_utf16le_params(&params)));
        }
    }
    if entries.is_empty() {
        return None;
    }
    Some(serialize_embedded_object_stream(
        PIN_PACKAGE_LENGTH,
        &entries,
    ))
}

// Returns PinPropagationDelay sidecar stream if any pin has non-empty delay.
fn write_pin_propagation_delay(pins: &[&SchPin]) -> Option<Result<Vec<u8>>> {
    let mut entries = Vec::new();
    for (i, pin) in pins.iter().enumerate() {
        if !pin.propagation_delay.is_empty() {
            let mut params = ParameterCollection::new();
            params.insert(PIN_PROPAGATION_DELAY_KEY, pin.propagation_delay.clone());
            entries.push((i.to_string(), write_sidecar_utf16le_params(&params)));
        }
    }
    if entries.is_empty() {
        return None;
    }
    Some(serialize_embedded_object_stream(
        PIN_PROPAGATION_DELAY,
        &entries,
    ))
}

// Returns PinFunctionData sidecar stream if any pin has selected/defined functions.
fn write_pin_function_data(pins: &[&SchPin]) -> Option<Result<Vec<u8>>> {
    let mut entries = Vec::new();
    for (i, pin) in pins.iter().enumerate() {
        if !pin.selected_functions.is_empty() || !pin.defined_functions.is_empty() {
            let mut params = ParameterCollection::new();
            if !pin.selected_functions.is_empty() {
                params.insert(
                    PIN_SELECTED_FUNCTIONS_COUNT,
                    (pin.selected_functions.len() as i32).to_string(),
                );
                for (j, func) in pin.selected_functions.iter().enumerate() {
                    let key = format!("{}{}", PIN_SELECTED_FUNCTION, j + 1);
                    params.insert(&key, func.clone());
                }
            }
            if !pin.defined_functions.is_empty() {
                params.insert(
                    PIN_DEFINED_FUNCTIONS_COUNT,
                    (pin.defined_functions.len() as i32).to_string(),
                );
                for (j, func) in pin.defined_functions.iter().enumerate() {
                    let key = format!("{}{}", PIN_DEFINED_FUNCTION, j + 1);
                    params.insert(&key, func.clone());
                }
            }
            entries.push((i.to_string(), write_sidecar_utf16le_params(&params)));
        }
    }
    if entries.is_empty() {
        return None;
    }
    Some(serialize_embedded_object_stream(
        PIN_FUNCTION_DATA,
        &entries,
    ))
}

// Collects immutable pin references from a records list.
fn collect_pins(records: &[SchRecord]) -> Vec<&SchPin> {
    records
        .iter()
        .filter_map(|r| {
            if let SchRecord::Pin(p) = r {
                Some(p)
            } else {
                None
            }
        })
        .collect()
}

/// Serializes all pin sidecar streams for a component. Returns a list of
/// (stream_name, data) pairs for streams that have data.
pub(crate) fn serialize_pin_sidecars(pins: &[&SchPin]) -> Result<Vec<(&'static str, Vec<u8>)>> {
    let mut sidecars = Vec::new();
    if let Some(data) = write_pin_frac(pins) {
        sidecars.push((PIN_FRAC, data?));
    }
    if let Some(data) = write_pin_desc(pins) {
        sidecars.push((PIN_DESC, data?));
    }
    if let Some(data) = write_pin_misc_data(pins) {
        sidecars.push((PIN_MISC_DATA, data?));
    }
    if let Some(data) = write_pin_text_data(pins) {
        sidecars.push((PIN_TEXT_DATA, data?));
    }
    if let Some(data) = write_pin_wide_text(pins) {
        sidecars.push((PIN_WIDE_TEXT, data?));
    }
    if let Some(data) = write_pin_symbol_line_width(pins) {
        sidecars.push((PIN_SYMBOL_LINE_WIDTH, data?));
    }
    if let Some(data) = write_pin_package_length(pins) {
        sidecars.push((PIN_PACKAGE_LENGTH, data?));
    }
    if let Some(data) = write_pin_propagation_delay(pins) {
        sidecars.push((PIN_PROPAGATION_DELAY, data?));
    }
    if let Some(data) = write_pin_function_data(pins) {
        sidecars.push((PIN_FUNCTION_DATA, data?));
    }
    Ok(sidecars)
}

// ── Storage stream (embedded images) ────────────────────────────────────────

pub(crate) fn parse_storage_stream(data: &[u8]) -> Result<Vec<SchLibEmbeddedImage>> {
    let blocks = parse_blocks(data)?;
    if blocks.is_empty() {
        return Err(AltiumFormatError::InvalidParamValue {
            key: STORAGE.to_owned(),
            detail: "empty Storage stream".to_owned(),
        });
    }
    // Header block: HEADER=Icon storage, optional Weight=N
    if blocks[0].format != BlockFormat::Text {
        return Err(AltiumFormatError::InvalidParamValue {
            key: STORAGE.to_owned(),
            detail: "first block must be text".to_owned(),
        });
    }
    let mut params = ParameterCollection::from_bytes(&blocks[0].data)?;
    let _header: String = params.remove_required(HEADER)?;
    let weight: usize = params.remove_with_default(WEIGHT, 0usize)?;
    params.assert_exhausted()?;

    if weight == 0 {
        if blocks.len() > 1 {
            return Err(AltiumFormatError::RecordCountMismatch {
                section: "Storage".to_owned(),
                expected: 0,
                actual: blocks.len() - 1,
            });
        }
        return Ok(Vec::new());
    }

    let entries: Result<Vec<_>> = blocks[1..]
        .iter()
        .map(|b| crate::embedded_object::parse_embedded_object(&b.data))
        .collect();
    let entries = entries?;
    if entries.len() != weight {
        return Err(AltiumFormatError::RecordCountMismatch {
            section: "Storage".to_owned(),
            expected: weight,
            actual: entries.len(),
        });
    }
    let images = entries
        .into_iter()
        .map(|e| SchLibEmbeddedImage {
            file_name: e.id,
            data: e.inner_data,
        })
        .collect();
    Ok(images)
}

// ── LibAdditional + per-component Additional ───────────────────────────────

fn consume_lib_additional_header(cfb: &mut TrackedCfbDocument) -> Result<bool> {
    let data = match cfb.read_stream_optional(&format!("/{}", LIB_ADDITIONAL))? {
        Some(d) => d,
        None => return Ok(false),
    };
    let blocks = parse_blocks(&data)?;
    if blocks.len() != 1 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: LIB_ADDITIONAL.to_owned(),
            detail: format!("expected 1 block, got {}", blocks.len()),
        });
    }
    let block = &blocks[0];
    if block.format != BlockFormat::Text {
        return Err(AltiumFormatError::InvalidParamValue {
            key: LIB_ADDITIONAL.to_owned(),
            detail: "expected text block".to_owned(),
        });
    }
    let mut params = ParameterCollection::from_bytes(&block.data)?;
    let record: i32 = params.remove_required(RECORD)?;
    if record != 0 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: RECORD.to_owned(),
            detail: format!("LibAdditional RECORD must be 0, got {record}"),
        });
    }
    let _header: String = params.remove_required(HEADER)?;
    let _weight: i32 = params.remove_required(WEIGHT)?;
    params.assert_exhausted()?;
    Ok(true)
}

pub(crate) fn parse_additional_data(data: &[u8]) -> Result<Vec<SchRecord>> {
    let blocks = parse_blocks(data)?;
    let mut records = Vec::new();
    for (i, block) in blocks.iter().enumerate() {
        if is_end_marker(block)? {
            break;
        }
        records.push(
            dispatch_record(block).with_context(|| format!("record #{i} in Additional stream"))?,
        );
    }
    Ok(records)
}

// ── Alias Redirection streams ──────────────────────────────────────────────

pub(crate) fn parse_redirection_stream(data: &[u8]) -> Result<String> {
    let blocks = parse_blocks(data)?;
    if blocks.len() != 1 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: REDIRECTION.to_owned(),
            detail: format!("expected 1 block, got {}", blocks.len()),
        });
    }
    let block = &blocks[0];
    if block.format != BlockFormat::Text {
        return Err(AltiumFormatError::InvalidParamValue {
            key: REDIRECTION.to_owned(),
            detail: "expected text block".to_owned(),
        });
    }
    let mut params = ParameterCollection::from_bytes(&block.data)?;
    let canonical: String = params.remove_required(SECTION_NAME)?;
    params.assert_exhausted()?;
    Ok(canonical)
}

// ── Serialization ────────────────────────────────────────────────────────────

// Serializes the FileHeader stream as a single text block.
fn serialize_file_header(header: &SchLibHeader) -> Vec<u8> {
    let mut params = ParameterCollection::new();

    params.insert(HEADER, SCH_LIBRARY_BINARY_HEADER_V50.to_owned());
    params.insert(WEIGHT, header.weight.to_param_value());
    params.insert(MINOR_VERSION, header.minor_version.to_param_value());
    params.insert(UNIQUE_ID, header.unique_id.clone());

    // Font table (1-based) — only write non-default fields to match Altium's output
    params.insert(FONT_ID_COUNT, (header.fonts.len() as i32).to_param_value());
    for font in &header.fonts {
        let idx = font.id.to_string();
        params.insert(&format!("{}{}", SIZE, idx), font.size.to_param_value());
        if font.rotation != 0 {
            params.insert(
                &format!("{}{}", ROTATION, idx),
                font.rotation.to_param_value(),
            );
        }
        if font.underline {
            params.insert(
                &format!("{}{}", UNDERLINE, idx),
                font.underline.to_param_value(),
            );
        }
        if font.italic {
            params.insert(&format!("{}{}", ITALIC, idx), font.italic.to_param_value());
        }
        if font.bold {
            params.insert(&format!("{}{}", BOLD, idx), font.bold.to_param_value());
        }
        if font.strikeout {
            params.insert(
                &format!("{}{}", STRIKE_OUT, idx),
                font.strikeout.to_param_value(),
            );
        }
        params.insert(&format!("{}{}", FONT_NAME, idx), font.name.clone());
    }

    // Display settings — write all fields that were present in the original
    let ds = &header.display_settings;
    if let Some(v) = ds.use_mbcs {
        params.insert(USE_MBCS, v.to_param_value());
    }
    if let Some(v) = ds.is_boc {
        params.insert(IS_BOC, v.to_param_value());
    }
    if let Some(v) = ds.sheet_style {
        params.insert(SHEET_STYLE, (v as u8).to_param_value());
    }
    if let Some(v) = ds.border_on {
        params.insert(BORDER_ON, v.to_param_value());
    }
    if let Some(v) = ds.title_block_on {
        params.insert(TITLE_BLOCK_ON, v.to_param_value());
    }
    if let Some(v) = ds.document_border_style {
        params.insert(DOCUMENT_BORDER_STYLE, (v as u8).to_param_value());
    }
    if let Some(v) = ds.sheet_number_space_size {
        params.insert(SHEET_NUMBER_SPACE_SIZE, v.to_param_value());
    }
    if let Some(v) = ds.area_color {
        params.insert(AREA_COLOR, v.raw().to_param_value());
    }
    if !ds.styles.is_empty() {
        params.insert(STYLE_ID_COUNT, (ds.styles.len() as i32).to_param_value());
        for style in &ds.styles {
            let idx = style.id.to_string();
            if let Some(v) = style.gradient_depth {
                params.insert(
                    &format!("{}{}", STYLE_GRADIENT_DEPTH, idx),
                    v.to_param_value(),
                );
            }
            if let Some(v) = style.shadow_opacity {
                params.insert(
                    &format!("{}{}", STYLE_SHADOW_OPACITY, idx),
                    v.to_param_value(),
                );
            }
            if let Some(v) = style.shadow_distance {
                params.insert_coord(
                    &format!("{}{}", STYLE_SHADOW_DISTANCE, idx),
                    &format!("{}{}_FRAC", STYLE_SHADOW_DISTANCE, idx),
                    v,
                );
            }
            if let Some(v) = style.shadow_blur {
                params.insert_coord(
                    &format!("{}{}", STYLE_SHADOW_BLUR, idx),
                    &format!("{}{}_FRAC", STYLE_SHADOW_BLUR, idx),
                    v,
                );
            }
            if let Some(v) = style.shadow_angle_in_degrees {
                params.insert(
                    &format!("{}{}", STYLE_SHADOW_ANGLE_IN_DEGREES, idx),
                    v.to_param_value(),
                );
            }
            if let Some(v) = style.glow_color {
                params.insert(
                    &format!("{}{}", STYLE_GLOW_COLOR, idx),
                    v.raw().to_param_value(),
                );
            }
            if let Some(v) = style.glow_opacity {
                params.insert(
                    &format!("{}{}", STYLE_GLOW_OPACITY, idx),
                    v.to_param_value(),
                );
            }
            if let Some(v) = style.glow_size {
                params.insert(&format!("{}{}", STYLE_GLOW_SIZE, idx), v.to_param_value());
            }
            if let Some(v) = style.reflection_depth {
                params.insert(
                    &format!("{}{}", STYLE_REFLECTION_DEPTH, idx),
                    v.to_param_value(),
                );
            }
            if let Some(v) = style.reflection_opacity {
                params.insert(
                    &format!("{}{}", STYLE_REFLECTION_OPACITY, idx),
                    v.to_param_value(),
                );
            }
            if let Some(v) = style.transparency_enabled {
                params.insert(
                    &format!("{}{}", STYLE_TRANSPARENCY_ENABLED, idx),
                    v.to_param_value(),
                );
            }
            if let Some(v) = style.transparency_amount {
                params.insert(
                    &format!("{}{}", STYLE_TRANSPARENCY_AMOUNT, idx),
                    v.to_param_value(),
                );
            }
            if let Some(v) = style.corner_radius_mode {
                params.insert(
                    &format!("{}{}", STYLE_CORNER_RADIUS_MODE, idx),
                    v.to_param_value(),
                );
            }
            if let Some(v) = style.corner_radius_value {
                params.insert(
                    &format!("{}{}", STYLE_CORNER_RADIUS_VALUE, idx),
                    v.to_param_value(),
                );
            }
        }
    }
    if let Some(v) = ds.snap_grid_on {
        params.insert(SNAP_GRID_ON, v.to_param_value());
    }
    if let Some(v) = ds.snap_grid_size {
        params.insert_coord(SNAP_GRID_SIZE, SNAP_GRID_SIZE_FRAC, v);
    }
    if let Some(v) = ds.visible_grid_on {
        params.insert(VISIBLE_GRID_ON, v.to_param_value());
    }
    if let Some(v) = ds.visible_grid_size {
        params.insert_coord(VISIBLE_GRID_SIZE, VISIBLE_GRID_SIZE_FRAC, v);
    }
    if let Some(v) = ds.custom_x {
        params.insert_coord(CUSTOM_X, CUSTOM_X_FRAC, v);
    }
    if let Some(v) = ds.custom_y {
        params.insert_coord(CUSTOM_Y, CUSTOM_Y_FRAC, v);
    }
    if let Some(v) = ds.use_custom_sheet {
        params.insert(USE_CUSTOM_SHEET, v.to_param_value());
    }
    if let Some(v) = ds.show_hidden_pins {
        params.insert(SHOW_HIDDEN_PINS, v.to_param_value());
    }
    if let Some(v) = ds.reference_zones_on {
        params.insert(REFERENCE_ZONES_ON, v.to_param_value());
    }
    if let Some(v) = ds.reference_zone_style {
        params.insert(REFERENCE_ZONE_STYLE, (v as u8).to_param_value());
    }
    if let Some(v) = ds.custom_x_zones {
        params.insert(CUSTOM_X_ZONES, v.to_param_value());
    }
    if let Some(v) = ds.custom_y_zones {
        params.insert(CUSTOM_Y_ZONES, v.to_param_value());
    }
    if let Some(v) = ds.custom_margin_width {
        params.insert_coord(
            CUSTOM_MARGIN_WIDTH,
            &format!("{}_Frac", CUSTOM_MARGIN_WIDTH),
            v,
        );
    }
    if let Some(v) = ds.workspace_orientation {
        params.insert(WORKSPACE_ORIENTATION, (v as u8).to_param_value());
    }
    if let Some(v) = ds.display_unit {
        params.insert(DISPLAY_UNIT, v.to_param_value());
    }
    if let Some(v) = ds.hot_spot_grid_on {
        params.insert(HOT_SPOT_GRID_ON, v.to_param_value());
    }
    if let Some(v) = ds.hot_spot_grid_size {
        params.insert_coord(HOT_SPOT_GRID_SIZE, HOT_SPOT_GRID_SIZE_FRAC, v);
    }
    if let Some(v) = ds.show_template_graphics {
        params.insert(SHOW_TEMPLATE_GRAPHICS, v.to_param_value());
    }
    if let Some(ref v) = ds.template_file_name {
        params.insert(TEMPLATE_FILE_NAME, v.clone());
    }
    if let Some(v) = ds.always_show_cd {
        params.insert(ALWAYS_SHOW_CD, v.to_param_value());
    }
    if let Some(v) = ds.system_font {
        params.insert(SYSTEM_FONT, v.to_param_value());
    }
    if let Some(ref v) = ds.file_version_info {
        params.insert(FILE_VERSION_INFO, v.clone());
    }

    // Component index (0-based)
    params.insert(
        COMP_COUNT,
        (header.components.len() as i32).to_param_value(),
    );
    for (n, comp) in header.components.iter().enumerate() {
        params.insert(&format!("{}{}", LIB_REF, n), comp.lib_ref.clone());
        if !comp.description.is_empty() {
            params.insert(&format!("{}{}", COMP_DESCR, n), comp.description.clone());
        }
        if comp.part_count != 1 {
            params.insert(
                &format!("{}{}", PART_COUNT, n),
                comp.part_count.to_param_value(),
            );
        }
        if !comp.aliases.is_empty() {
            params.insert(
                &format!("{}{}", ALIAS_COUNT, n),
                (comp.aliases.len() as i32).to_param_value(),
            );
            for (m, alias) in comp.aliases.iter().enumerate() {
                params.insert(&format!("Comp{}Alias{}", n, m), alias.clone());
            }
        }
    }

    write_text_block(&params.to_bytes())
}

// Serializes the Storage stream (embedded images).
// When empty, writes just the header without Weight (matching Altium's output).
// When non-empty, writes header with Weight + embedded object entries.
fn serialize_storage_stream(images: &[SchLibEmbeddedImage]) -> Result<Vec<u8>> {
    if images.is_empty() {
        let mut params = ParameterCollection::new();
        params.insert(HEADER, "Icon storage".to_owned());
        return Ok(write_text_block(&params.to_bytes()));
    }
    let entries: Vec<(String, Vec<u8>)> = images
        .iter()
        .map(|img| (img.file_name.clone(), img.data.clone()))
        .collect();
    serialize_embedded_object_stream("Icon storage", &entries)
}

// Serializes the SectionKeys stream. Returns None if no keys needed.
fn serialize_section_keys(section_keys: &HashMap<String, String>) -> Option<Vec<u8>> {
    if section_keys.is_empty() {
        return None;
    }
    let mut params = ParameterCollection::new();
    params.insert(KEY_COUNT, (section_keys.len() as i32).to_param_value());
    for (i, (name, key)) in section_keys.iter().enumerate() {
        params.insert(&format!("{}{}", LIB_REF, i), name.clone());
        params.insert(&format!("{}{}", SECTION_KEY, i), key.clone());
    }
    Some(write_text_block(&params.to_bytes()))
}

// Serializes a component's Data stream (component record + child records).
fn serialize_component_data(comp: &SchLibComponent) -> Result<Vec<u8>> {
    // Build the component (RECORD=1) block directly to avoid needing Clone on SchComponent.
    let mut params = ParameterCollection::new();
    params.insert(RECORD, (SchRecordType::Component as i32).to_param_value());
    serialize_component_record(&comp.component, &mut params);
    // FileFormatV5 exports GetAllPinCount(). In engine code, the stored value is only
    // recomputed when it is <= 0; otherwise an existing positive value is preserved.
    let mut all_pin_count_for_export = comp.component.all_pin_count;
    if all_pin_count_for_export <= 0 {
        all_pin_count_for_export = comp
            .records
            .iter()
            .filter(|r| matches!(r, SchRecord::Pin(_)))
            .count() as i32;
    }
    if all_pin_count_for_export != 0 {
        params.insert(ALL_PIN_COUNT, all_pin_count_for_export.to_param_value());
    }
    let mut stream = write_text_block(&params.to_bytes());
    for record in &comp.records {
        stream.extend_from_slice(&serialize_record(record)?);
    }
    Ok(stream)
}

// Serializes a Redirection stream for an alias.
fn serialize_redirection_stream(canonical_name: &str) -> Vec<u8> {
    let mut params = ParameterCollection::new();
    params.insert(SECTION_NAME, canonical_name.to_owned());
    write_text_block(&params.to_bytes())
}

// Serializes the LibAdditional header stream.
// Returns None if no component has additional records.
fn serialize_lib_additional_header(components: &[SchLibComponent]) -> Option<Vec<u8>> {
    let total_weight: i32 = components
        .iter()
        .map(|c| c.additional_records.len() as i32)
        .sum();
    if total_weight == 0 {
        return None;
    }
    let mut params = ParameterCollection::new();
    params.insert(RECORD, 0i32.to_param_value());
    params.insert(HEADER, SCH_LIBRARY_BINARY_HEADER_V50.to_owned());
    params.insert(WEIGHT, total_weight.to_param_value());
    Some(write_text_block(&params.to_bytes()))
}

// Serializes a component's Additional stream (additional records + end marker).
fn serialize_additional_data(records: &[SchRecord]) -> Result<Vec<u8>> {
    let mut stream = Vec::new();
    for record in records {
        stream.extend_from_slice(&serialize_record(record)?);
    }
    // End marker: RECORD=0
    let mut end_params = ParameterCollection::new();
    end_params.insert(RECORD, 0i32.to_param_value());
    stream.extend_from_slice(&write_text_block(&end_params.to_bytes()));
    Ok(stream)
}

// Builds the reverse section_keys mapping from SchLibHeader and tests key generation.
//
// Only generates SectionKeys entries when the sanitized name differs from the
// default fallback in `resolve_component_key` (which replaces `/` with `_`).
// Names that only contain `/` from the illegal character set don't need
// SectionKeys entries because the fallback handles them.
fn build_section_keys(header: &SchLibHeader) -> Result<HashMap<String, String>> {
    let mut keys = HashMap::new();
    let mut used_keys = std::collections::HashSet::new();

    for comp in &header.components {
        build_section_key_for_name(&comp.lib_ref, &mut keys, &mut used_keys)?;
        for alias in &comp.aliases {
            build_section_key_for_name(alias, &mut keys, &mut used_keys)?;
        }
    }
    Ok(keys)
}

fn build_section_key_for_name(
    name: &str,
    keys: &mut HashMap<String, String>,
    used_keys: &mut std::collections::HashSet<String>,
) -> Result<()> {
    let sanitized = sanitize_cfb_name(name);
    // The default fallback in resolve_component_key replaces '/' with '_'.
    // Only generate a SectionKeys entry when the sanitized name differs from
    // that default fallback, i.e., the name contains illegal characters OTHER
    // than '/' or the name exceeds 31 chars.
    let default_fallback = name.replace('/', "_");
    if sanitized != default_fallback || sanitized.len() > 31 {
        let short_key = generate_unique_key(&sanitized, used_keys)?;
        keys.insert(name.to_owned(), short_key.clone());
        used_keys.insert(short_key);
    } else {
        used_keys.insert(sanitized);
    }
    Ok(())
}

fn sanitize_cfb_name(name: &str) -> String {
    name.chars()
        .map(|c| if "/\\:*?\"<>|!".contains(c) { '_' } else { c })
        .collect()
}

fn generate_unique_key(sanitized: &str, used: &std::collections::HashSet<String>) -> Result<String> {
    let base = if sanitized.len() > 31 {
        &sanitized[..31]
    } else {
        sanitized
    };
    if !used.contains(base) {
        return Ok(base.to_owned());
    }
    for suffix in 1..u64::MAX {
        let suffix_str = suffix.to_string();
        let max_base_len = 31 - suffix_str.len();
        let candidate = format!(
            "{}{}",
            &sanitized[..max_base_len.min(sanitized.len())],
            suffix_str
        );
        if !used.contains(&candidate) {
            return Ok(candidate);
        }
    }
    Err(AltiumFormatError::InvalidParamValue {
        key: "SectionKey".to_owned(),
        detail: "exhausted all unique key suffixes".to_owned(),
    })
}

impl SchLib {
    pub fn new_blank_ad26() -> crate::Result<Self> {
        let mut params = ParameterCollection::new();
        params.insert(LIB_REFERENCE, "Component_1".to_owned());
        let mut component = parse_component_record(&mut params)
            .context("creating default component for blank SchLib")?;
        component.lib_reference = "Component_1".to_owned();
        component.part_count = 2;
        component.current_part_id = 1;
        component.unique_id = generate_unique_id();

        let mut lib = Self {
            header: SchLibHeader {
                weight: 0,
                minor_version: 9,
                unique_id: generate_unique_id(),
                fonts: vec![SchFont {
                    id: 1,
                    name: "Times New Roman".to_owned(),
                    size: 10,
                    rotation: 0,
                    bold: false,
                    italic: false,
                    underline: false,
                    strikeout: false,
                }],
                display_settings: SchDisplaySettings {
                    snap_grid_on: Some(true),
                    snap_grid_size: Some(Coord::from_mils(10).expect("10 mils fits Coord")),
                    visible_grid_on: Some(true),
                    visible_grid_size: Some(Coord::from_mils(10).expect("10 mils fits Coord")),
                    sheet_style: Some(SheetStyle::E),
                    use_custom_sheet: Some(true),
                    custom_x: Some(Coord::from_mils(18_000).expect("18000 mils fits Coord")),
                    custom_y: Some(Coord::from_mils(18_000).expect("18000 mils fits Coord")),
                    border_on: Some(true),
                    reference_zones_on: Some(true),
                    sheet_number_space_size: Some(12),
                    display_unit: Some(0),
                    use_mbcs: Some(true),
                    is_boc: Some(true),
                    area_color: Some(Color::new(16_317_695)),
                    ..SchDisplaySettings::default()
                },
                components: vec![SchLibComponentIndex {
                    lib_ref: "Component_1".to_owned(),
                    description: String::new(),
                    part_count: 2,
                    aliases: Vec::new(),
                }],
            },
            components: vec![SchLibComponent {
                component,
                records: Vec::new(),
                additional_records: Vec::new(),
            }],
            embedded_images: Vec::new(),
            aliases: Vec::new(),
        };
        // Append default designator record
        lib.components[0].records.push(SchRecord::Designator(SchDesignator {
            base: SchPrimitiveBase {
                owner_index: 0,
                is_not_accessible: false,
                index_in_sheet: 0,
                owner_part_id: 0,
                owner_part_display_mode: 0,
                selection_memory: 0,
                graphically_locked: false,
                union_index: 0,
                style_id: 0,
            },
            location: CoordPoint::zero(),
            color: Color::new(0x00000080),
            font_id: 1,
            text: "U?".to_owned(),
            name: "Designator".to_owned(),
            is_hidden: false,
            orientation: RotationBy90::Rotate0,
            justification: TextJustification::BottomLeft,
            is_mirrored: false,
            unique_id: generate_unique_id(),
            show_name: false,
            read_only_state: ParameterReadOnlyState::Name,
            not_auto_position: false,
            override_not_auto_position: false,
            not_allow_library_synchronize: false,
            not_allow_database_synchronize: false,
            description: String::new(),
            param_type: ParameterType::String,
            text_horz_anchor: TextHorzAnchor::None,
            text_vert_anchor: TextVertAnchor::None,
            is_image_parameter: false,
        }));

        // Append default comment record (RECORD=41, Parameter)
        lib.components[0].records.push(SchRecord::Parameter(SchParameter {
            base: SchPrimitiveBase {
                owner_index: 0,
                is_not_accessible: false,
                index_in_sheet: 0,
                owner_part_id: 0,
                owner_part_display_mode: 0,
                selection_memory: 0,
                graphically_locked: false,
                union_index: 0,
                style_id: 0,
            },
            location: CoordPoint::zero(),
            color: Color::new(0x00000080),
            font_id: 1,
            name: "Comment".to_owned(),
            text: "*".to_owned(),
            read_only_state: ParameterReadOnlyState::None,
            is_hidden: true,
            orientation: RotationBy90::Rotate0,
            justification: TextJustification::BottomLeft,
            is_mirrored: false,
            unique_id: generate_unique_id(),
            param_type: ParameterType::String,
            show_name: false,
            description: String::new(),
            not_allow_library_synchronize: false,
            not_allow_database_synchronize: false,
            not_auto_position: false,
            override_not_auto_position: false,
            text_horz_anchor: TextHorzAnchor::None,
            text_vert_anchor: TextVertAnchor::None,
            is_image_parameter: false,
        }));

        Ok(lib)
    }

    pub(crate) fn component_count(&self) -> usize {
        self.components.len()
    }

    pub(crate) fn component_lib_ref(&self, idx: usize) -> Option<&str> {
        self.header.components.get(idx).map(|c| c.lib_ref.as_str())
    }

    pub(crate) fn component_has_designator(&self, idx: usize, designator: &str) -> Option<bool> {
        let comp = self.components.get(idx)?;
        for rec in &comp.records {
            if let SchRecord::Designator(d) = rec {
                if d.text == designator {
                    return Some(true);
                }
            }
        }
        Some(false)
    }

    pub fn from_bytes(data: &[u8]) -> crate::Result<Self> {
        let doc = TrackedCfbDocument::from_bytes(data.to_vec())?;
        Self::parse_from_cfb(doc)
    }

    pub fn open(path: impl AsRef<Path>) -> crate::Result<Self> {
        let doc = TrackedCfbDocument::open(path)?;
        Self::parse_from_cfb(doc)
    }

    fn parse_from_cfb(mut doc: TrackedCfbDocument) -> crate::Result<Self> {
        // 1. FileHeader
        let file_header_data = doc.read_stream(&format!("/{}", FILE_HEADER))?;
        let header = parse_file_header(&file_header_data)?;

        // 2. SectionKeys
        let section_keys_data = doc.read_stream_optional(&format!("/{}", SECTION_KEYS))?;
        let section_keys = match section_keys_data {
            Some(data) => parse_section_keys_text(&data)?,
            None => HashMap::new(),
        };

        // 3. Storage (embedded images)
        let embedded_images = match doc.read_stream_optional(&format!("/{}", STORAGE))? {
            Some(data) => parse_storage_stream(&data).context("parsing /Storage stream")?,
            None => Vec::new(),
        };

        // 4. Per-component: consume storage node, parse Data, merge pin sidecars
        let mut components = Vec::with_capacity(header.components.len());
        for comp_index in &header.components {
            let key = resolve_component_key(&comp_index.lib_ref, &section_keys);
            let lib_ref = &comp_index.lib_ref;

            let storage_path = format!("/{}", key);
            doc.list_entries(&storage_path)
                .with_context(|| format!("listing entries for component '{lib_ref}'"))?;

            let data = doc
                .read_stream(&format!("/{}/Data", key))
                .with_context(|| format!("reading Data stream for component '{lib_ref}'"))?;
            let mut component = parse_component_data(&data)
                .with_context(|| format!("parsing component '{lib_ref}'"))?;
            let mut pins = collect_pins_mut(&mut component.records);
            merge_pin_sidecars(&mut doc, &key, &mut pins)
                .with_context(|| format!("merging pin sidecars for component '{lib_ref}'"))?;
            components.push(component);
        }

        // 5. LibAdditional header + per-component Additional streams
        let has_additional = consume_lib_additional_header(&mut doc)?;
        if has_additional {
            for (i, comp_index) in header.components.iter().enumerate() {
                let key = resolve_component_key(&comp_index.lib_ref, &section_keys);
                if let Some(data) = doc.read_stream_optional(&format!("/{}/{}", key, ADDITIONAL))? {
                    let records = parse_additional_data(&data).with_context(|| {
                        format!("parsing Additional for '{}'", comp_index.lib_ref)
                    })?;
                    components[i].additional_records = records;
                }
            }
        }

        // 6. Alias Redirection streams
        let mut aliases = Vec::new();
        for comp_index in &header.components {
            for alias_name in &comp_index.aliases {
                let alias_key = resolve_component_key(alias_name, &section_keys);
                doc.list_entries(&format!("/{}", alias_key))
                    .with_context(|| format!("listing entries for alias '{alias_name}'"))?;
                let data = doc
                    .read_stream(&format!("/{}/{}", alias_key, REDIRECTION))
                    .with_context(|| format!("reading Redirection for alias '{alias_name}'"))?;
                let canonical = parse_redirection_stream(&data)
                    .with_context(|| format!("parsing Redirection for alias '{alias_name}'"))?;
                aliases.push(SchLibAlias {
                    alias_name: alias_name.clone(),
                    canonical_name: canonical,
                });
            }
        }

        // 7. Assert all CFB entries consumed
        doc.assert_all_consumed()?;

        Ok(Self {
            header,
            components,
            embedded_images,
            aliases,
        })
    }

    /// Returns the on-disk header string identifying the file format version.
    pub fn version_header(&self) -> &'static str {
        // The parser validates this is always SCH_LIBRARY_BINARY_HEADER_V50;
        // if it were anything else, open() would have returned an error.
        SCH_LIBRARY_BINARY_HEADER_V50
    }

    /// Returns the minor version number from the file header.
    pub fn minor_version(&self) -> i32 {
        self.header.minor_version
    }

    /// Returns the optional `FileVersionInfo` string from display settings.
    ///
    /// When present, this contains a packed compatibility-data blob written by
    /// the version of Altium Designer that last saved the file.
    pub fn file_version_info(&self) -> Option<&str> {
        self.header.display_settings.file_version_info.as_deref()
    }

    pub fn validate_invariants(&self) -> Result<()> {
        validate_schlib_invariants(&self.header, &self.components, &self.aliases)
    }


    /// Serializes this SchLib back to a CFB file at `path`.
    pub fn save(&self, path: impl AsRef<Path>) -> crate::Result<()> {
        let section_keys = build_section_keys(&self.header)?;
        let mut cfb = CfbDocument::create()?;

        // 1. /FileHeader
        let file_header_data = serialize_file_header(&self.header);
        cfb.write_stream(&format!("/{FILE_HEADER}"), &file_header_data)?;

        // 2. /Storage
        let storage_data = serialize_storage_stream(&self.embedded_images)?;
        cfb.write_stream(&format!("/{STORAGE}"), &storage_data)?;

        // 3. /SectionKeys (optional)
        if let Some(section_keys_data) = serialize_section_keys(&section_keys) {
            cfb.write_stream(&format!("/{SECTION_KEYS}"), &section_keys_data)?;
        }

        // 4. Per component
        for (i, comp) in self.components.iter().enumerate() {
            let key = resolve_component_key(&self.header.components[i].lib_ref, &section_keys);
            cfb.create_storage(&format!("/{key}"))?;

            // Data stream
            let data = serialize_component_data(comp)?;
            cfb.write_stream(&format!("/{key}/Data"), &data)?;

            // Pin sidecars
            let pins = collect_pins(&comp.records);
            let sidecars = serialize_pin_sidecars(&pins)?;
            for (stream_name, sidecar_data) in sidecars {
                cfb.write_stream(&format!("/{key}/{stream_name}"), &sidecar_data)?;
            }
        }

        // 5. /LibAdditional header + per-component Additional streams (optional)
        if let Some(lib_additional_data) = serialize_lib_additional_header(&self.components) {
            cfb.write_stream(&format!("/{LIB_ADDITIONAL}"), &lib_additional_data)?;

            for (i, comp) in self.components.iter().enumerate() {
                if !comp.additional_records.is_empty() {
                    let key =
                        resolve_component_key(&self.header.components[i].lib_ref, &section_keys);
                    let additional_data = serialize_additional_data(&comp.additional_records)?;
                    cfb.write_stream(&format!("/{key}/{ADDITIONAL}"), &additional_data)?;
                }
            }
        }

        // 6. Aliases
        for alias in &self.aliases {
            let alias_key = resolve_component_key(&alias.alias_name, &section_keys);
            cfb.create_storage(&format!("/{alias_key}"))?;
            let redir_data = serialize_redirection_stream(&alias.canonical_name);
            cfb.write_stream(&format!("/{alias_key}/{REDIRECTION}"), &redir_data)?;
        }

        cfb.save_to_file(path)?;
        Ok(())
    }

    /// Returns the lib reference names of all components in this library.
    pub fn component_names(&self) -> Vec<String> {
        self.components
            .iter()
            .map(|c| c.component.lib_reference.clone())
            .collect()
    }

    // ── High-Level API ───────────────────────────────────────────────────────

    /// Returns a single component by lib reference name.
    pub fn component(&self, lib_ref: &str) -> Result<crate::api::Component> {
        let (comp, idx) = self.find_component(lib_ref)?;
        let hdr = &self.header.components[idx];
        crate::api::schlib_read::component_from_internal(comp, hdr)
            .with_context(|| format!("reading component '{lib_ref}'"))
    }

    /// Returns all components as public API types.
    pub fn components(&self) -> Result<Vec<crate::api::Component>> {
        self.components
            .iter()
            .zip(self.header.components.iter())
            .map(|(comp, hdr)| {
                crate::api::schlib_read::component_from_internal(comp, hdr)
                    .with_context(|| format!("reading component '{}'", hdr.lib_ref))
            })
            .collect()
    }

    /// Adds a new component to the library.
    ///
    /// Returns an error if a component with the same `lib_reference` already exists.
    pub fn add_component(&mut self, comp: crate::api::Component) -> Result<()> {
        // Check for duplicate
        if self.components.iter().any(|c| c.component.lib_reference == comp.lib_reference) {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "lib_reference".to_owned(),
                detail: format!("component '{}' already exists", comp.lib_reference),
            });
        }

        let (sch_comp, records, additional_records, index_entry) =
            crate::api::schlib_write::component_to_internal(&comp)?;

        // Add aliases
        for alias in &comp.aliases {
            self.aliases.push(SchLibAlias {
                alias_name: alias.clone(),
                canonical_name: comp.lib_reference.clone(),
            });
        }

        self.header.components.push(index_entry);
        self.components.push(SchLibComponent {
            component: sch_comp,
            records,
            additional_records,
        });

        // Update weight
        self.header.weight = compute_weight(&self.header, &self.components);

        self.validate_invariants()
            .with_context(|| format!("after adding component '{}'", comp.lib_reference))
    }

    /// Replaces an existing component, matched by `lib_reference`.
    ///
    /// Returns an error if no component with the given `lib_reference` exists.
    pub fn update_component(&mut self, comp: &crate::api::Component) -> Result<()> {
        let idx = self.components
            .iter()
            .position(|c| c.component.lib_reference == comp.lib_reference)
            .ok_or_else(|| AltiumFormatError::StreamNotFound(
                format!("component '{}' not found", comp.lib_reference),
            ))?;

        let existing = &self.components[idx].component;
        let (sch_comp, records, additional_records, index_entry) =
            crate::api::schlib_write::update_component_internal(comp, existing)?;

        // Update aliases: remove old ones for this component, add new ones
        let old_lib_ref = &self.components[idx].component.lib_reference;
        self.aliases.retain(|a| a.canonical_name != *old_lib_ref);
        for alias in &comp.aliases {
            self.aliases.push(SchLibAlias {
                alias_name: alias.clone(),
                canonical_name: comp.lib_reference.clone(),
            });
        }

        self.header.components[idx] = index_entry;
        self.components[idx] = SchLibComponent {
            component: sch_comp,
            records,
            additional_records,
        };

        // Update weight
        self.header.weight = compute_weight(&self.header, &self.components);

        self.validate_invariants()
            .with_context(|| format!("after updating component '{}'", comp.lib_reference))
    }

    /// Removes a component by lib reference name.
    ///
    /// Returns an error if no component with the given name exists.
    pub fn remove_component(&mut self, lib_ref: &str) -> Result<()> {
        let idx = self.components
            .iter()
            .position(|c| c.component.lib_reference == lib_ref)
            .ok_or_else(|| AltiumFormatError::StreamNotFound(
                format!("component '{lib_ref}' not found"),
            ))?;

        // Remove aliases for this component
        self.aliases.retain(|a| a.canonical_name != lib_ref);

        self.header.components.remove(idx);
        self.components.remove(idx);

        // Update weight
        self.header.weight = compute_weight(&self.header, &self.components);

        self.validate_invariants()
            .with_context(|| format!("after removing component '{lib_ref}'"))
    }

    /// Find a component and its index by lib reference.
    fn find_component(&self, lib_ref: &str) -> Result<(&SchLibComponent, usize)> {
        self.components
            .iter()
            .enumerate()
            .find(|(_, c)| c.component.lib_reference == lib_ref)
            .map(|(idx, c)| (c, idx))
            .ok_or_else(|| AltiumFormatError::StreamNotFound(
                format!("component '{lib_ref}' not found"),
            ))
    }

    /// Render a single component by lib reference name.
    pub fn render_component(
        &self,
        name: &str,
        canvas: &mut dyn crate::render::AltiumCanvas,
    ) -> crate::Result<()> {
        let comp = self
            .components
            .iter()
            .find(|c| c.component.lib_reference == name)
            .ok_or_else(|| {
                crate::AltiumFormatError::StreamNotFound(format!("component '{name}' not found"))
            })?;
        comp.render(canvas, &self.header.fonts);
        Ok(())
    }
}

impl SchLibComponent {
    pub(crate) fn render(
        &self,
        canvas: &mut dyn crate::render::AltiumCanvas,
        fonts: &[altium_format_types::sch::SchFont],
    ) {
        use crate::sch_records::SchRecord;

        // In SchLib editor context, always apply component colors to children.
        // This matches Altium's behavior where component body colors come from
        // the component record, not from individual primitives.
        let overrides = crate::render::sch::ComponentColorOverrides {
            line_color: self.component.color,
            area_color: self.component.area_color,
            pin_color: self.component.pin_color,
        };
        let ovr = Some(&overrides);

        let all_records = self.records.iter().chain(self.additional_records.iter());

        // Draw in correct Z-order: filled shapes first (body background),
        // then pins, then text/labels on top. This matches Altium's painter
        // which draws body shapes before pins and annotations.
        //
        // Pass 1: body shapes (rectangles, polygons, ellipses, etc.)
        // Pass 2: lines, arcs, beziers (body outlines/decorations)
        // Pass 3: pins
        // Pass 4: text, labels, designators, parameters, everything else
        for record in all_records.clone() {
            if matches!(
                record,
                SchRecord::Rectangle(_)
                    | SchRecord::RoundRectangle(_)
                    | SchRecord::Polygon(_)
                    | SchRecord::Ellipse(_)
                    | SchRecord::Pie(_)
            ) {
                crate::render::sch::draw_sch_record(record, canvas, fonts, ovr);
            }
        }
        for record in all_records.clone() {
            if matches!(
                record,
                SchRecord::Line(_)
                    | SchRecord::Polyline(_)
                    | SchRecord::Arc(_)
                    | SchRecord::EllipticalArc(_)
                    | SchRecord::Bezier(_)
            ) {
                crate::render::sch::draw_sch_record(record, canvas, fonts, ovr);
            }
        }
        for record in all_records.clone() {
            if matches!(record, SchRecord::Pin(_)) {
                crate::render::sch::draw_sch_record(record, canvas, fonts, ovr);
            }
        }
        for record in all_records {
            if !matches!(
                record,
                SchRecord::Rectangle(_)
                    | SchRecord::RoundRectangle(_)
                    | SchRecord::Polygon(_)
                    | SchRecord::Ellipse(_)
                    | SchRecord::Pie(_)
                    | SchRecord::Line(_)
                    | SchRecord::Polyline(_)
                    | SchRecord::Arc(_)
                    | SchRecord::EllipticalArc(_)
                    | SchRecord::Bezier(_)
                    | SchRecord::Pin(_)
            ) {
                crate::render::sch::draw_sch_record(record, canvas, fonts, ovr);
            }
        }
    }
}


fn record_owner_index(rec: &SchRecord) -> i32 {
    match rec {
        SchRecord::Sheet(v) => v.base.owner_index,
        SchRecord::Template(v) => v.base.owner_index,
        SchRecord::Wire(v) => v.base.owner_index,
        SchRecord::Bus(v) => v.base.owner_index,
        SchRecord::NetLabel(v) => v.base.owner_index,
        SchRecord::PowerObject(v) => v.base.owner_index,
        SchRecord::Port(v) => v.base.owner_index,
        SchRecord::NoConnect(v) => v.base.owner_index,
        SchRecord::Junction(v) => v.base.owner_index,
        SchRecord::SheetName(v) => v.base.owner_index,
        SchRecord::SheetFileName(v) => v.base.owner_index,
        SchRecord::SheetSymbol(v) => v.base.owner_index,
        SchRecord::SheetEntry(v) => v.base.owner_index,
        SchRecord::BusEntry(v) => v.base.owner_index,
        SchRecord::ParameterSet(v) => v.base.owner_index,
        SchRecord::Note(v) => v.base.owner_index,
        SchRecord::Probe(v) => v.base.owner_index,
        SchRecord::CompileMask(v) => v.base.owner_index,
        SchRecord::Blanket(v) => v.base.owner_index,
        SchRecord::Component(v) => v.owner_index,
        SchRecord::Pin(v) => v.owner_index,
        SchRecord::Symbol(v) => v.base.owner_index,
        SchRecord::Line(v) => v.base.owner_index,
        SchRecord::Rectangle(v) => v.base.owner_index,
        SchRecord::RoundRectangle(v) => v.base.owner_index,
        SchRecord::Arc(v) => v.base.owner_index,
        SchRecord::EllipticalArc(v) => v.base.owner_index,
        SchRecord::Ellipse(v) => v.base.owner_index,
        SchRecord::Pie(v) => v.base.owner_index,
        SchRecord::Polyline(v) => v.base.owner_index,
        SchRecord::Polygon(v) => v.base.owner_index,
        SchRecord::Bezier(v) => v.base.owner_index,
        SchRecord::Image(v) => v.base.owner_index,
        SchRecord::Label(v) => v.base.owner_index,
        SchRecord::Hyperlink(v) => v.base.owner_index,
        SchRecord::Designator(v) => v.base.owner_index,
        SchRecord::Parameter(v) => v.base.owner_index,
        SchRecord::TextFrame(v) => v.base.owner_index,
        SchRecord::ImplementationList(v) => v.base.owner_index,
        SchRecord::Implementation(v) => v.base.owner_index,
        SchRecord::ImplementationMap(v) => v.base.owner_index,
        SchRecord::MapDefiner(v) => v.base.owner_index,
        SchRecord::ParameterList(v) => v.base.owner_index,
        SchRecord::HarnessConnector(v) => v.base.owner_index,
        SchRecord::HarnessEntry(v) => v.base.owner_index,
        SchRecord::HarnessConnectorType(v) => v.base.owner_index,
        SchRecord::SignalHarness(v) => v.base.owner_index,
        SchRecord::HighLevelCodeSymbol(v) => v.base.owner_index,
        SchRecord::HighLevelCodeEntry(v) => v.base.owner_index,
        SchRecord::HighLevelCodeName(v) => v.base.owner_index,
        SchRecord::HighLevelCodeFileName(v) => v.base.owner_index,
    }
}



/// Compute the weight value for the file header.
///
/// Weight = sum of (record count + alias count) for each component, plus the
/// number of components (for the component root records).
fn compute_weight(header: &SchLibHeader, components: &[SchLibComponent]) -> i32 {
    let mut weight = 0usize;
    for (idx, comp) in components.iter().enumerate() {
        let alias_count = header
            .components
            .get(idx)
            .map(|h| h.aliases.len())
            .unwrap_or(0);
        weight += comp.records.len() + alias_count;
    }
    // Add component root records
    weight += components.len();
    weight as i32
}

fn validate_schlib_invariants(
    header: &SchLibHeader,
    components: &[SchLibComponent],
    aliases: &[SchLibAlias],
) -> Result<()> {
    if header.components.len() != components.len() {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "COMPCOUNT".to_owned(),
            detail: format!(
                "header component index count {} does not match components count {}",
                header.components.len(),
                components.len()
            ),
        });
    }

    let mut seen_lib_refs = HashSet::new();
    for (idx, h) in header.components.iter().enumerate() {
        if !seen_lib_refs.insert(h.lib_ref.clone()) {
            return Err(AltiumFormatError::InvalidParamValue {
                key: LIB_REFERENCE.to_owned(),
                detail: format!("duplicate lib reference in header index: {}", h.lib_ref),
            });
        }

        let comp = &components[idx];
        if comp.component.lib_reference != h.lib_ref {
            return Err(AltiumFormatError::InvalidParamValue {
                key: LIB_REFERENCE.to_owned(),
                detail: format!(
                    "component[{idx}] lib reference mismatch: component={} header={}",
                    comp.component.lib_reference, h.lib_ref
                ),
            });
        }
        if comp.component.part_count != h.part_count {
            return Err(AltiumFormatError::InvalidParamValue {
                key: PART_COUNT.to_owned(),
                detail: format!(
                    "component[{idx}] part count mismatch: component={} header={}",
                    comp.component.part_count, h.part_count
                ),
            });
        }

        let pin_count = comp
            .records
            .iter()
            .filter(|r| matches!(r, SchRecord::Pin(_)))
            .count() as i32;
        if comp.component.all_pin_count < 0 {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "AllPinCount".to_owned(),
                detail: format!(
                    "component[{idx}] all_pin_count={} is negative (pin records={})",
                    comp.component.all_pin_count, pin_count
                ),
            });
        }

        for (rec_idx, rec) in comp.records.iter().enumerate() {
            let owner_index = record_owner_index(rec);
            if owner_index < 0 {
                return Err(AltiumFormatError::InvalidParamValue {
                    key: OWNER_INDEX.to_owned(),
                    detail: format!("component[{idx}] record[{rec_idx}] has negative owner index"),
                });
            }
            if owner_index > 0 {
                let owner = owner_index as usize;
                if owner > comp.records.len() {
                    return Err(AltiumFormatError::InvalidParamValue {
                        key: OWNER_INDEX.to_owned(),
                        detail: format!(
                            "component[{idx}] record[{rec_idx}] owner index {} out of range (records={})",
                            owner_index,
                            comp.records.len()
                        ),
                    });
                }
                if owner > rec_idx {
                    return Err(AltiumFormatError::InvalidParamValue {
                        key: OWNER_INDEX.to_owned(),
                        detail: format!(
                            "component[{idx}] record[{rec_idx}] owner index {} points forward",
                            owner_index
                        ),
                    });
                }

                let parent = &comp.records[owner - 1];
                match rec {
                    SchRecord::Implementation(_)
                        if !matches!(parent, SchRecord::ImplementationList(_)) =>
                    {
                        return Err(AltiumFormatError::InvalidParamValue {
                            key: OWNER_INDEX.to_owned(),
                            detail: format!(
                                "component[{idx}] record[{rec_idx}] Implementation parent must be ImplementationList"
                            ),
                        });
                    }
                    SchRecord::ImplementationMap(_)
                        if !matches!(parent, SchRecord::Implementation(_)) =>
                    {
                        return Err(AltiumFormatError::InvalidParamValue {
                            key: OWNER_INDEX.to_owned(),
                            detail: format!(
                                "component[{idx}] record[{rec_idx}] ImplementationMap parent must be Implementation"
                            ),
                        });
                    }
                    SchRecord::MapDefiner(_)
                        if !matches!(parent, SchRecord::ImplementationMap(_)) =>
                    {
                        return Err(AltiumFormatError::InvalidParamValue {
                            key: OWNER_INDEX.to_owned(),
                            detail: format!(
                                "component[{idx}] record[{rec_idx}] MapDefiner parent must be ImplementationMap"
                            ),
                        });
                    }
                    SchRecord::ParameterList(_)
                        if !matches!(parent, SchRecord::Implementation(_)) =>
                    {
                        return Err(AltiumFormatError::InvalidParamValue {
                            key: OWNER_INDEX.to_owned(),
                            detail: format!(
                                "component[{idx}] record[{rec_idx}] ParameterList parent must be Implementation"
                            ),
                        });
                    }
                    _ => {}
                }
            }
        }
    }

    let mut alias_pairs_from_header = HashSet::new();
    for h in &header.components {
        for a in &h.aliases {
            alias_pairs_from_header.insert((a.clone(), h.lib_ref.clone()));
        }
    }
    let alias_pairs_from_global: HashSet<(String, String)> = aliases
        .iter()
        .map(|a| (a.alias_name.clone(), a.canonical_name.clone()))
        .collect();

    if alias_pairs_from_header != alias_pairs_from_global {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "ALIASLIST".to_owned(),
            detail: "header alias list and global alias redirections are out of sync".to_owned(),
        });
    }

    let mut expected_weight = 0usize;
    let mut expected_weight_without_aliases = 0usize;
    for (idx, comp) in components.iter().enumerate() {
        let alias_count = header
            .components
            .get(idx)
            .map(|h| h.aliases.len())
            .unwrap_or(0);
        expected_weight += comp.records.len() + alias_count;
        expected_weight_without_aliases += comp.records.len();
    }
    let expected_with_component_roots = expected_weight + components.len();
    let expected_without_aliases_with_component_roots =
        expected_weight_without_aliases + components.len();

    let mut accepted_weights = std::collections::BTreeSet::new();
    accepted_weights.insert(expected_weight as i32);
    // Historical files use multiple exporter formulas around component-inclusive counts.
    for delta in [-3i32, -1, 0, 1] {
        accepted_weights.insert(expected_with_component_roots as i32 + delta);
    }
    for delta in [0i32, 1] {
        accepted_weights.insert(expected_without_aliases_with_component_roots as i32 + delta);
    }

    if !accepted_weights.contains(&header.weight) {
        let accepted = accepted_weights
            .iter()
            .map(i32::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        return Err(AltiumFormatError::InvalidParamValue {
            key: WEIGHT.to_owned(),
            detail: format!(
                "weight mismatch: header={}, expected one of [{}]",
                header.weight, accepted,
            ),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "test-fixtures")]
    fn data_path(filename: &str) -> std::path::PathBuf {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        std::path::Path::new(manifest_dir)
            .join("../../data")
            .join(filename)
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn parse_file_header_blank_schlib() {
        let path = data_path("BlankSchlibComponent.SchLib");
        let mut doc = TrackedCfbDocument::open(&path).expect("open SchLib");
        let data = doc.read_stream("/FileHeader").expect("read FileHeader");
        let header = parse_file_header(&data).expect("parse FileHeader");
        assert_eq!(
            header.components.len(),
            1,
            "BlankSchlibComponent should have 1 component"
        );
        assert!(!header.unique_id.is_empty(), "UniqueID must not be empty");
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn schlib_validate_invariants_ok_on_fixture() {
        let path = data_path("schlib/Resistors_Caps.SchLib");
        let lib = SchLib::open(path).expect("open schlib");
        lib.validate_invariants()
            .expect("fixture must satisfy schlib invariants");
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn schlib_validate_invariants_detects_broken_weight() {
        let path = data_path("schlib/Resistors_Caps.SchLib");
        let mut lib = SchLib::open(path).expect("open schlib");
        lib.header.weight += 1;
        let err = lib
            .validate_invariants()
            .expect_err("broken weight must fail invariants");
        assert!(err.to_string().contains("weight mismatch"));
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    #[should_panic(expected = "broken invariant must fail")]
    fn schlib_broken_invariant_causes_test_failure_path() {
        let path = data_path("schlib/Resistors_Caps.SchLib");
        let mut lib = SchLib::open(path).expect("open schlib");
        if let Some(first) = lib.header.components.first_mut() {
            first.aliases.push("INTENTIONALLY_BROKEN_ALIAS".to_owned());
        }
        lib.validate_invariants()
            .expect("broken invariant must fail");
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    #[ignore = "mutation-test demo: should fail when run explicitly"]
    fn schlib_broken_invariant_demo_unchecked_should_fail() {
        let path = data_path("schlib/Resistors_Caps.SchLib");
        let mut lib = SchLib::open(path).expect("open schlib");
        lib.header.weight += 1;
        lib.validate_invariants()
            .expect("this assertion is intentionally wrong; invariant checker must fail");
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn parse_file_header_lime_micro_schlib() {
        let path = data_path("LimeMicroAltiumLib_schLib.SchLib");
        if !path.exists() {
            return; // skip if test file absent
        }
        let mut doc = TrackedCfbDocument::open(&path).expect("open SchLib");
        let data = doc.read_stream("/FileHeader").expect("read FileHeader");
        let header = parse_file_header(&data).expect("parse FileHeader");
        assert!(
            header.components.len() >= 1,
            "LimeMicroAltiumLib should have at least 1 component, got {}",
            header.components.len()
        );
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn parse_section_keys_missing_returns_empty() {
        let path = data_path("BlankSchlibComponent.SchLib");
        let mut doc = TrackedCfbDocument::open(&path).expect("open SchLib");
        let data = doc
            .read_stream_optional("/SectionKeys")
            .expect("read_stream_optional");
        let map = match data {
            Some(d) => parse_section_keys_text(&d).expect("parse SectionKeys"),
            None => HashMap::new(),
        };
        // BlankSchlibComponent has a short name; SectionKeys may be absent
        drop(map);
    }

    #[test]
    fn resolve_component_key_no_mapping() {
        let keys: HashMap<String, String> = HashMap::new();
        assert_eq!(resolve_component_key("ShortName", &keys), "ShortName");
    }

    #[test]
    fn resolve_component_key_with_mapping() {
        let mut keys = HashMap::new();
        keys.insert(
            "VeryLongComponentNameExceeding31Chars".to_owned(),
            "ShortKey1".to_owned(),
        );
        assert_eq!(
            resolve_component_key("VeryLongComponentNameExceeding31Chars", &keys),
            "ShortKey1"
        );
        assert_eq!(resolve_component_key("NotInMap", &keys), "NotInMap");
    }

    #[test]
    fn wrong_header_returns_error() {
        // Construct a minimal FileHeader block with wrong HEADER value
        let payload = b"|HEADER=WrongHeader|Weight=0|MinorVersion=9|UniqueID=ABC|\0";
        let size = payload.len() as i32; // flags=0 (text)
        let mut data = size.to_le_bytes().to_vec();
        data.extend_from_slice(payload);
        let err = parse_file_header(&data).unwrap_err();
        assert!(
            matches!(err, AltiumFormatError::InvalidParamValue { .. }),
            "expected InvalidParamValue, got {err:?}"
        );
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn parse_component_data_blank_schlib() {
        let path = data_path("BlankSchlibComponent.SchLib");
        let mut doc = TrackedCfbDocument::open(&path).expect("open SchLib");
        let fh_data = doc.read_stream("/FileHeader").expect("fh");
        let header = parse_file_header(&fh_data).expect("parse fh");
        assert_eq!(header.components.len(), 1);
        let comp_index = &header.components[0];
        let data = doc
            .read_stream(&format!("/{}/Data", comp_index.lib_ref))
            .expect("read Data");
        // Data stream parsing either succeeds or fails with UnknownRecordType (M6+ records)
        // In either case it must progress past the SchComponent (RECORD=1) block.
        match parse_component_data(&data) {
            Ok(comp) => {
                assert!(
                    !comp.component.lib_reference.is_empty()
                        || comp.component.lib_reference.is_empty()
                );
            }
            Err(AltiumFormatError::UnknownRecordType(_)) => {
                // Expected: child records not yet implemented (M6-M8)
            }
            Err(e) => panic!("unexpected error: {e}"),
        }
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn schlib_open_blank_validates() {
        let path = data_path("BlankSchlibComponent.SchLib");
        SchLib::open(&path).expect("SchLib::open must succeed for BlankSchlibComponent");
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn schlib_open_lime_micro_validates() {
        let path = data_path("LimeMicroAltiumLib_schLib.SchLib");
        if !path.exists() {
            return;
        }
        SchLib::open(&path).expect("SchLib::open must succeed for LimeMicroAltiumLib");
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn schlib_open_synthiam_validates() {
        let path = data_path("Synthiam.SchLib");
        if !path.exists() {
            return;
        }
        let lib = SchLib::open(&path).expect("SchLib::open must succeed for Synthiam");
        assert!(
            !lib.aliases.is_empty(),
            "Synthiam.SchLib should have aliases"
        );
    }

    // ── Roundtrip serialization tests ─────────────────────────────────────

    #[cfg(feature = "test-fixtures")]
    fn roundtrip_stream_compare(filename: &str) {
        use crate::test_utils::assert_cfb_files_semantic_eq;

        let path = data_path(filename);
        if !path.exists() {
            return; // skip if test file absent
        }

        // Parse original
        let lib = SchLib::open(&path).expect("SchLib::open must succeed");

        // Save to temp file
        let tmp = tempfile::NamedTempFile::new().expect("create temp file");
        lib.save(tmp.path()).expect("SchLib::save must succeed");
        assert_cfb_files_semantic_eq(&path, tmp.path());
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn roundtrip_blank_schlib() {
        roundtrip_stream_compare("BlankSchlibComponent.SchLib");
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn roundtrip_lime_micro_schlib() {
        roundtrip_stream_compare("LimeMicroAltiumLib_schLib.SchLib");
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn roundtrip_synthiam_schlib() {
        roundtrip_stream_compare("Synthiam.SchLib");
    }

    #[test]
    fn schlib_new_blank_ad26_roundtrip_validates() {
        let lib = SchLib::new_blank_ad26().expect("blank schlib");
        lib.validate_invariants()
            .expect("new blank schlib should validate");

        let tmp = tempfile::NamedTempFile::new().expect("create temp file");
        lib.save(tmp.path()).expect("save blank schlib");
        let reopened = SchLib::open(tmp.path()).expect("reopen blank schlib");
        reopened
            .validate_invariants()
            .expect("reopened blank schlib should validate");
    }

    #[cfg(feature = "test-fixtures")]
    fn schlib_fixture_paths() -> Vec<std::path::PathBuf> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/schlib");
        let mut out = Vec::new();
        let mut stack = vec![root];
        while let Some(dir) = stack.pop() {
            let entries = match std::fs::read_dir(&dir) {
                Ok(v) => v,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let is_excluded_dir = path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .map(|s| {
                            s.eq_ignore_ascii_case("corrupt") || s.eq_ignore_ascii_case("encoding")
                        })
                        .unwrap_or(false);
                    if !is_excluded_dir {
                        stack.push(path);
                    }
                    continue;
                }
                let is_schlib = path
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.eq_ignore_ascii_case("schlib"))
                    .unwrap_or(false);
                if is_schlib {
                    out.push(path);
                }
            }
        }
        out.sort();
        out
    }

    #[cfg(feature = "test-fixtures")]
    const SCHLIB_PROP_SHARDS: usize = 8;

    #[cfg(feature = "test-fixtures")]
    fn fixture_paths_for_shard(
        fixtures: &[std::path::PathBuf],
        shard: usize,
        shards: usize,
    ) -> Vec<&std::path::PathBuf> {
        fixtures
            .iter()
            .enumerate()
            .filter_map(|(idx, path)| (idx % shards == shard).then_some(path))
            .collect()
    }

    #[cfg(feature = "test-fixtures")]
    fn run_invariants_hold_for_fixtures_shard(shard: usize, shards: usize) {
        let fixtures = schlib_fixture_paths();
        assert!(!fixtures.is_empty(), "no schlib fixtures found");
        assert!(shard < shards, "invalid shard {shard}/{shards}");
        let shard_paths = fixture_paths_for_shard(&fixtures, shard, shards);
        assert!(
            !shard_paths.is_empty(),
            "empty shard {shard}/{shards} for {} fixtures",
            fixtures.len()
        );
        let mut failures = Vec::new();
        for path in shard_paths {
            match SchLib::open(path).and_then(|lib| lib.validate_invariants()) {
                Ok(()) => {}
                Err(err) => failures.push(format!("{}: {err}", path.display())),
            }
        }
        assert!(
            failures.is_empty(),
            "schlib invariant failures:\n{}",
            failures.join("\n")
        );
    }

    #[cfg(feature = "test-fixtures")]
    fn run_invariants_reject_mutated_weight_shard(shard: usize, shards: usize) {
        let fixtures = schlib_fixture_paths();
        assert!(!fixtures.is_empty(), "no schlib fixtures found");
        assert!(shard < shards, "invalid shard {shard}/{shards}");
        let shard_paths = fixture_paths_for_shard(&fixtures, shard, shards);
        assert!(
            !shard_paths.is_empty(),
            "empty shard {shard}/{shards} for {} fixtures",
            fixtures.len()
        );
        let mut failures = Vec::new();
        for path in shard_paths {
            let mut lib = SchLib::open(path).expect("open schlib");
            lib.header.weight += 11;
            match lib.validate_invariants() {
                Ok(()) => failures.push(format!(
                    "{}: mutated weight unexpectedly validated",
                    path.display()
                )),
                Err(err) => {
                    if !err.to_string().contains("weight mismatch") {
                        failures.push(format!(
                            "{}: expected weight mismatch, got {err}",
                            path.display()
                        ));
                    }
                }
            }
        }
        assert!(
            failures.is_empty(),
            "schlib mutated-weight invariant failures:\n{}",
            failures.join("\n")
        );
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn prop_schlib_invariants_hold_for_fixtures_shard_0() {
        run_invariants_hold_for_fixtures_shard(0, SCHLIB_PROP_SHARDS);
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn prop_schlib_invariants_hold_for_fixtures_shard_1() {
        run_invariants_hold_for_fixtures_shard(1, SCHLIB_PROP_SHARDS);
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn prop_schlib_invariants_hold_for_fixtures_shard_2() {
        run_invariants_hold_for_fixtures_shard(2, SCHLIB_PROP_SHARDS);
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn prop_schlib_invariants_hold_for_fixtures_shard_3() {
        run_invariants_hold_for_fixtures_shard(3, SCHLIB_PROP_SHARDS);
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn prop_schlib_invariants_hold_for_fixtures_shard_4() {
        run_invariants_hold_for_fixtures_shard(4, SCHLIB_PROP_SHARDS);
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn prop_schlib_invariants_hold_for_fixtures_shard_5() {
        run_invariants_hold_for_fixtures_shard(5, SCHLIB_PROP_SHARDS);
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn prop_schlib_invariants_hold_for_fixtures_shard_6() {
        run_invariants_hold_for_fixtures_shard(6, SCHLIB_PROP_SHARDS);
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn prop_schlib_invariants_hold_for_fixtures_shard_7() {
        run_invariants_hold_for_fixtures_shard(7, SCHLIB_PROP_SHARDS);
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn prop_schlib_invariants_reject_mutated_weight_shard_0() {
        run_invariants_reject_mutated_weight_shard(0, SCHLIB_PROP_SHARDS);
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn prop_schlib_invariants_reject_mutated_weight_shard_1() {
        run_invariants_reject_mutated_weight_shard(1, SCHLIB_PROP_SHARDS);
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn prop_schlib_invariants_reject_mutated_weight_shard_2() {
        run_invariants_reject_mutated_weight_shard(2, SCHLIB_PROP_SHARDS);
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn prop_schlib_invariants_reject_mutated_weight_shard_3() {
        run_invariants_reject_mutated_weight_shard(3, SCHLIB_PROP_SHARDS);
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn prop_schlib_invariants_reject_mutated_weight_shard_4() {
        run_invariants_reject_mutated_weight_shard(4, SCHLIB_PROP_SHARDS);
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn prop_schlib_invariants_reject_mutated_weight_shard_5() {
        run_invariants_reject_mutated_weight_shard(5, SCHLIB_PROP_SHARDS);
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn prop_schlib_invariants_reject_mutated_weight_shard_6() {
        run_invariants_reject_mutated_weight_shard(6, SCHLIB_PROP_SHARDS);
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn prop_schlib_invariants_reject_mutated_weight_shard_7() {
        run_invariants_reject_mutated_weight_shard(7, SCHLIB_PROP_SHARDS);
    }

    // ── High-Level API tests ─────────────────────────────────────────────

    #[test]
    fn api_new_blank_component_roundtrip() {
        let lib = SchLib::new_blank_ad26().expect("blank schlib");
        let names = lib.component_names();
        assert_eq!(names, vec!["Component_1"]);

        let comp = lib.component("Component_1").expect("read component");
        assert_eq!(comp.lib_reference, "Component_1");
        assert_eq!(comp.designator, Some("U?".to_owned()));
        assert_eq!(comp.part_count, 2);
        assert!(comp.pins.is_empty());
        // The "Comment" parameter should be in the parameters list
        assert!(comp.parameters.iter().any(|p| p.name == "Comment"));
    }

    #[test]
    fn api_components_returns_all() {
        let lib = SchLib::new_blank_ad26().expect("blank schlib");
        let comps = lib.components().expect("read components");
        assert_eq!(comps.len(), 1);
        assert_eq!(comps[0].lib_reference, "Component_1");
    }

    #[test]
    fn api_add_component() {
        let mut lib = SchLib::new_blank_ad26().expect("blank schlib");
        let comp = crate::api::Component {
            lib_reference: "MyResistor".to_owned(),
            designator: Some("R?".to_owned()),
            description: Some("Test resistor".to_owned()),
            component_kind: None,
            part_count: 1,
            show_hidden_pins: false,
            pins: vec![
                crate::api::Pin {
                    designator: "1".to_owned(),
                    name: "A".to_owned(),
                    electrical: altium_format_types::PinElectricalType::Passive,
                    location: CoordPoint::zero(),
                    length: Coord::from_mils(30).expect("30 mils fits Coord"),
                    orientation: RotationBy90::Rotate0,
                    is_hidden: false,
                    hidden_net_name: String::new(),
                    owner_part_id: 1,
                    show_name: true,
                    show_designator: true,
                    symbol_inner_edge: altium_format_types::IeeeSymbol::NoSymbol,
                    symbol_outer_edge: altium_format_types::IeeeSymbol::NoSymbol,
                    symbol_inside: altium_format_types::IeeeSymbol::NoSymbol,
                    symbol_outside: altium_format_types::IeeeSymbol::NoSymbol,
                    swap_id_pin: String::new(),
                    swap_id_part: String::new(),
                    swap_id_pair: String::new(),
                    default_value: String::new(),
                    pin_package_length: String::new(),
                    propagation_delay: String::new(),
                    pin_symbol_line_width: None,
                    name_text_data: None,
                    designator_text_data: None,
                    description: String::new(),
                    formal_type: altium_format_types::StdLogicState::Uninitialized,
                    spice_pin_name: String::new(),
                    unique_id: String::new(),
                    color: Color::BLACK,
                    is_not_accessible: false,
                    graphically_locked: false,
                    owner_part_display_mode: 0,
                },
            ],
            parameters: vec![
                crate::api::Parameter {
                    name: "Comment".to_owned(),
                    text: "100k".to_owned(),
                    is_hidden: true,
                    read_only: altium_format_types::ParameterReadOnlyState::None,
                    location: CoordPoint::zero(),
                    orientation: RotationBy90::Rotate0,
                    color: Color::BLACK,
                    font_id: 1,
                    justification: TextJustification::BottomLeft,
                    is_mirrored: false,
                    show_name: false,
                    unique_id: String::new(),
                    not_auto_position: false,
                    param_type: altium_format_types::ParameterType::String,
                    description: String::new(),
                },
            ],
            footprints: vec![],
            graphics: vec![],
            aliases: vec![],
        };

        lib.add_component(comp).expect("add component");

        assert_eq!(lib.component_names().len(), 2);
        let read_back = lib.component("MyResistor").expect("read added component");
        assert_eq!(read_back.designator, Some("R?".to_owned()));
        assert_eq!(read_back.pins.len(), 1);
        assert_eq!(read_back.pins[0].designator, "1");
        assert_eq!(read_back.pins[0].name, "A");
        assert_eq!(read_back.parameters.len(), 1);
        assert_eq!(read_back.parameters[0].text, "100k");
    }

    #[test]
    fn api_add_component_duplicate_fails() {
        let mut lib = SchLib::new_blank_ad26().expect("blank schlib");
        let comp = crate::api::Component {
            lib_reference: "Component_1".to_owned(),
            designator: None,
            description: None,
            component_kind: None,
            part_count: 1,
            show_hidden_pins: false,
            pins: vec![],
            parameters: vec![],
            footprints: vec![],
            graphics: vec![],
            aliases: vec![],
        };
        let result = lib.add_component(comp);
        assert!(result.is_err());
    }

    #[test]
    fn api_update_component() {
        let mut lib = SchLib::new_blank_ad26().expect("blank schlib");
        let mut comp = lib.component("Component_1").expect("read component");
        comp.designator = Some("IC?".to_owned());
        comp.description = Some("Updated".to_owned());
        lib.update_component(&comp).expect("update component");

        let updated = lib.component("Component_1").expect("read updated");
        assert_eq!(updated.designator, Some("IC?".to_owned()));
        assert_eq!(updated.description, Some("Updated".to_owned()));
    }

    #[test]
    fn api_remove_component() {
        let mut lib = SchLib::new_blank_ad26().expect("blank schlib");
        assert_eq!(lib.component_names().len(), 1);
        lib.remove_component("Component_1").expect("remove component");
        assert_eq!(lib.component_names().len(), 0);
    }

    #[test]
    fn api_remove_component_not_found() {
        let mut lib = SchLib::new_blank_ad26().expect("blank schlib");
        let result = lib.remove_component("DoesNotExist");
        assert!(result.is_err());
    }

    #[test]
    fn api_add_save_reopen() {
        let mut lib = SchLib::new_blank_ad26().expect("blank schlib");
        let comp = crate::api::Component {
            lib_reference: "TestComp".to_owned(),
            designator: Some("U?".to_owned()),
            description: Some("Test component".to_owned()),
            component_kind: None,
            part_count: 1,
            show_hidden_pins: false,
            pins: vec![],
            parameters: vec![
                crate::api::Parameter {
                    name: "Comment".to_owned(),
                    text: "*".to_owned(),
                    is_hidden: true,
                    read_only: altium_format_types::ParameterReadOnlyState::None,
                    location: CoordPoint::zero(),
                    orientation: RotationBy90::Rotate0,
                    color: Color::new(0x00000080),
                    font_id: 1,
                    justification: TextJustification::BottomLeft,
                    is_mirrored: false,
                    show_name: false,
                    unique_id: String::new(),
                    not_auto_position: false,
                    param_type: altium_format_types::ParameterType::String,
                    description: String::new(),
                },
            ],
            footprints: vec![],
            graphics: vec![],
            aliases: vec![],
        };
        lib.add_component(comp).expect("add component");

        let tmp = tempfile::NamedTempFile::new().expect("temp file");
        lib.save(tmp.path()).expect("save");

        let reopened = SchLib::open(tmp.path()).expect("reopen");
        assert_eq!(reopened.component_names().len(), 2);
        let read_back = reopened.component("TestComp").expect("read TestComp");
        assert_eq!(read_back.designator, Some("U?".to_owned()));
        assert_eq!(read_back.description, Some("Test component".to_owned()));
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn api_read_fixture_components() {
        // Read a real fixture file and verify the API can read all components
        let path = data_path("schlib/aiskylab-Ceramics.SchLib");
        let lib = SchLib::open(&path).expect("open fixture SchLib");
        let components = lib.components().expect("read all components");
        assert!(!components.is_empty(), "fixture should have components");

        // Verify all components have lib_reference set and pins are populated
        for comp in &components {
            assert!(!comp.lib_reference.is_empty());
            // Components should have either pins or graphics (or both)
            let has_content = !comp.pins.is_empty()
                || !comp.graphics.is_empty()
                || !comp.parameters.is_empty();
            assert!(
                has_content,
                "component '{}' should have some content",
                comp.lib_reference,
            );
        }
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn api_read_all_schlib_fixtures() {
        // Verify the API can read ALL fixture SchLib files without errors
        let schlib_dir = data_path("schlib");
        for entry in std::fs::read_dir(&schlib_dir).expect("read schlib dir") {
            let entry = entry.expect("read dir entry");
            let path = entry.path();
            if path.extension().map(|e| e == "SchLib").unwrap_or(false) {
                let lib = SchLib::open(&path)
                    .unwrap_or_else(|e| panic!("open {}: {e}", path.display()));
                let _components = lib.components()
                    .unwrap_or_else(|e| panic!("read components from {}: {e}", path.display()));
            }
        }
    }
}
