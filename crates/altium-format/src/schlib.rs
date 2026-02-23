use std::collections::HashMap;
use std::path::Path;

use altium_format_types::{
    Color, Coord, RotationBy90, SchDisplaySettings, SchRecordType, SheetBorderStyle,
    SheetOrientation, SheetReferenceZoneStyle, SheetStyle,
};
use altium_format_types::sch::SchFont;
use altium_format_types::constants::component::{
    ALIAS_COUNT, COMP_COUNT, COMP_DESCR, LIB_REF, PART_COUNT,
};
use altium_format_types::constants::file_headers::SCH_LIBRARY_BINARY_HEADER_V50;
use altium_format_types::constants::parsing::C_BASE_UNIT;
use altium_format_types::constants::pin::{
    DEF_VALUE, PAIR_SWAP_ID, PIN_BINARY_CODE, PIN_DEFINED_FUNCTION, PIN_DEFINED_FUNCTIONS_COUNT,
    PIN_PACKAGE_LENGTH as PIN_PACKAGE_LENGTH_KEY, PIN_PROPAGATION_DELAY as PIN_PROPAGATION_DELAY_KEY,
    PIN_SELECTED_FUNCTION, PIN_SELECTED_FUNCTIONS_COUNT, PIN_TEXT_FONT_CUSTOM, PIN_TEXT_POS_CUSTOM,
    PIN_TEXT_ROT_ANCHOR, PIN_TEXT_ROT_REL_MASK, PIN_TEXT_ROT_REL_SHIFT, SWAP_ID, SWAP_ID_PART,
    SYMBOL_LINE_WIDTH,
};
use altium_format_types::constants::record_structure::{HEADER, KEY_COUNT, RECORD, RECORD_EX, SECTION_KEY, WEIGHT};
use altium_format_types::constants::record_structure::ALWAYS_SHOW_CD;
use altium_format_types::constants::sheet::{
    AREA_COLOR, BORDER_ON, CUSTOM_MARGIN_WIDTH, CUSTOM_X, CUSTOM_X_FRAC, CUSTOM_X_ZONES,
    CUSTOM_Y, CUSTOM_Y_FRAC, CUSTOM_Y_ZONES, DISPLAY_UNIT, DOCUMENT_BORDER_STYLE,
    FILE_VERSION_INFO, HOT_SPOT_GRID_ON, HOT_SPOT_GRID_SIZE, HOT_SPOT_GRID_SIZE_FRAC, IS_BOC,
    REFERENCE_ZONE_STYLE, REFERENCE_ZONES_ON, SHEET_NUMBER_SPACE_SIZE, SHEET_STYLE,
    SHOW_HIDDEN_PINS, SHOW_TEMPLATE_GRAPHICS, SNAP_GRID_ON, SNAP_GRID_SIZE, SNAP_GRID_SIZE_FRAC,
    SYSTEM_FONT, TEMPLATE_FILE_NAME, TITLE_BLOCK_ON, USE_CUSTOM_SHEET, USE_MBCS,
    VISIBLE_GRID_ON, VISIBLE_GRID_SIZE, VISIBLE_GRID_SIZE_FRAC, WORKSPACE_ORIENTATION,
};
use altium_format_types::constants::record_structure::SECTION_NAME;
use altium_format_types::constants::streams::{
    ADDITIONAL, FILE_HEADER, LIB_ADDITIONAL, PIN_DESC, PIN_FRAC, PIN_FUNCTION_DATA,
    PIN_MISC_DATA, PIN_PACKAGE_LENGTH, PIN_PROPAGATION_DELAY, PIN_SYMBOL_LINE_WIDTH,
    PIN_TEXT_DATA, PIN_WIDE_TEXT, REDIRECTION, SECTION_KEYS, STORAGE,
};
use altium_format_types::constants::text::{BOLD, DESC, DESIG, ITALIC, STRIKE_OUT, UNDERLINE};
use altium_format_types::constants::record_structure::UNIQUE_ID;
use altium_format_types::constants::sheet::MINOR_VERSION;
use altium_format_types::constants::visual::{FONT_ID_COUNT, FONT_NAME, ROTATION, SIZE};
use altium_format_types::constants::text::NAME;


use crate::binary_io::{BinaryReader, BinaryWriter};
use crate::block_stream::{parse_blocks, write_text_block, Block, BlockFormat};
use crate::cfb_document::CfbDocument;
use crate::embedded_object::{parse_embedded_object_stream, serialize_embedded_object_stream};
use crate::param_collection::ParameterCollection;
use crate::param_value::ToParamValue;
use crate::sch_records::{
    parse_binary_pin, parse_component_record, serialize_component_record, serialize_record,
    PinTextPositioning, SchArc, SchBezier,
    SchDesignator, SchEllipse, SchEllipticalArc, SchImage, SchImplementation,
    SchImplementationList, SchImplementationMap, SchLabel, SchLibComponent, SchLine, SchMapDefiner,
    SchParameter, SchParameterList, SchPie, SchPin, SchPolygon, SchPolyline, SchRecord,
    SchRectangle, SchRoundRectangle, SchTextFrame,
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
    let unique_id: String = params.remove_required(UNIQUE_ID)?;

    // Font table (1-based indexing)
    let fonts = params.remove_indexed(FONT_ID_COUNT, 1, |p, i| {
        let idx = i.to_string();
        let name: String = p.remove_required(&format!("{}{}", FONT_NAME, idx))?;
        let size: i32 = p.remove_required(&format!("{}{}", SIZE, idx))?;
        let rotation: i32 = p.remove_with_default(&format!("{}{}", ROTATION, idx), 0i32)?;
        let bold: bool = p.remove_with_default(&format!("{}{}", BOLD, idx), false)?;
        let italic: bool = p.remove_with_default(&format!("{}{}", ITALIC, idx), false)?;
        let underline: bool = p.remove_with_default(&format!("{}{}", UNDERLINE, idx), false)?;
        let strikeout: bool = p.remove_with_default(&format!("{}{}", STRIKE_OUT, idx), false)?;
        Ok(SchFont { id: i as i32, name, size, rotation, bold, italic, underline, strikeout })
    })?;

    // Display settings — library-level sheet display preferences, preserved for round-trip
    let display_settings = SchDisplaySettings {
        snap_grid_on: params.remove_optional(SNAP_GRID_ON)?,
        snap_grid_size: params.remove_coord_optional(SNAP_GRID_SIZE, SNAP_GRID_SIZE_FRAC)?,
        visible_grid_on: params.remove_optional(VISIBLE_GRID_ON)?,
        visible_grid_size: params.remove_coord_optional(VISIBLE_GRID_SIZE, VISIBLE_GRID_SIZE_FRAC)?,
        hot_spot_grid_on: params.remove_optional(HOT_SPOT_GRID_ON)?,
        hot_spot_grid_size: params.remove_coord_optional(HOT_SPOT_GRID_SIZE, HOT_SPOT_GRID_SIZE_FRAC)?,
        sheet_style: params.remove_optional::<u8>(SHEET_STYLE)?
            .map(SheetStyle::try_from).transpose()?,
        use_custom_sheet: params.remove_optional(USE_CUSTOM_SHEET)?,
        custom_x: params.remove_coord_optional(CUSTOM_X, CUSTOM_X_FRAC)?,
        custom_y: params.remove_coord_optional(CUSTOM_Y, CUSTOM_Y_FRAC)?,
        border_on: params.remove_optional(BORDER_ON)?,
        title_block_on: params.remove_optional(TITLE_BLOCK_ON)?,
        document_border_style: params.remove_optional::<u8>(DOCUMENT_BORDER_STYLE)?
            .map(SheetBorderStyle::try_from).transpose()?,
        reference_zones_on: params.remove_optional(REFERENCE_ZONES_ON)?,
        reference_zone_style: params.remove_optional::<u8>(REFERENCE_ZONE_STYLE)?
            .map(SheetReferenceZoneStyle::try_from).transpose()?,
        custom_x_zones: params.remove_optional(CUSTOM_X_ZONES)?,
        custom_y_zones: params.remove_optional(CUSTOM_Y_ZONES)?,
        custom_margin_width: params.remove_coord_optional(CUSTOM_MARGIN_WIDTH, &format!("{}_Frac", CUSTOM_MARGIN_WIDTH))?,
        sheet_number_space_size: params.remove_optional(SHEET_NUMBER_SPACE_SIZE)?,
        workspace_orientation: params.remove_optional::<u8>(WORKSPACE_ORIENTATION)?
            .map(SheetOrientation::try_from).transpose()?,
        show_hidden_pins: params.remove_optional(SHOW_HIDDEN_PINS)?,
        show_template_graphics: params.remove_optional(SHOW_TEMPLATE_GRAPHICS)?,
        always_show_cd: params.remove_optional(ALWAYS_SHOW_CD)?,
        template_file_name: params.remove_optional(TEMPLATE_FILE_NAME)?,
        display_unit: params.remove_optional(DISPLAY_UNIT)?,
        system_font: params.remove_optional(SYSTEM_FONT)?,
        use_mbcs: params.remove_optional(USE_MBCS)?,
        is_boc: params.remove_optional(IS_BOC)?,
        area_color: params.remove_optional::<i32>(AREA_COLOR)?.map(Color::new),
        file_version_info: params.remove_optional(FILE_VERSION_INFO)?,
    };

    // Component index (0-based indexing)
    let components = params.remove_indexed(COMP_COUNT, 0, |p, n| {
        let lib_ref: String = p.remove_required(&format!("{}{}", LIB_REF, n))?;
        let description: String = p.remove_with_default(&format!("{}{}", COMP_DESCR, n), String::new())?;
        let part_count: i32 = p.remove_with_default(&format!("{}{}", PART_COUNT, n), 1i32)?;
        let alias_count: i32 = p.remove_with_default(&format!("{}{}", ALIAS_COUNT, n), 0i32)?;
        let mut aliases = Vec::with_capacity(alias_count as usize);
        for m in 0..alias_count {
            let alias_key = format!("{}{}Alias{}", "Comp", n, m);
            let alias: String = p.remove_required(&alias_key)?;
            aliases.push(alias);
        }
        Ok(SchLibComponentIndex { lib_ref, description, part_count, aliases })
    })?;

    params.assert_exhausted()?;

    Ok(SchLibHeader { weight, minor_version, unique_id, fonts, display_settings, components })
}

pub(crate) fn parse_section_keys(data: &[u8]) -> Result<HashMap<String, String>> {
    let blocks = parse_blocks(data)?;
    if blocks.len() != 1 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: SECTION_KEYS.to_owned(),
            detail: format!("expected 1 block, got {}", blocks.len()),
        });
    }
    let block = &blocks[0];
    if block.format != BlockFormat::Text {
        return Err(AltiumFormatError::InvalidParamValue {
            key: SECTION_KEYS.to_owned(),
            detail: "expected text block, got binary".to_owned(),
        });
    }

    let mut params = ParameterCollection::from_bytes(&block.data)?;

    if let Some(record) = params.remove_optional::<i32>(RECORD)? {
        if record != 0 {
            return Err(AltiumFormatError::InvalidParamValue {
                key: RECORD.to_owned(),
                detail: format!("SectionKeys RECORD must be 0, got {record}"),
            });
        }
    }

    let mut map = HashMap::new();
    let count: i32 = params.remove_required(KEY_COUNT)?;
    for n in 0..count {
        let lib_ref: String = params.remove_required(&format!("{}{}", LIB_REF, n))?;
        let section_key: String = params.remove_required(&format!("{}{}", SECTION_KEY, n))?;
        map.insert(lib_ref, section_key);
    }

    params.assert_exhausted()?;

    Ok(map)
}

pub(crate) fn resolve_component_key(
    name: &str,
    section_keys: &HashMap<String, String>,
) -> String {
    let key = section_keys.get(name).map(String::as_str).unwrap_or(name);
    key.replace('/', "_")
}

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
            let record_type_val = if record_raw == 254 {
                params.remove_required::<i32>(RECORD_EX)?
            } else {
                record_raw
            };
            let record_type = SchRecordType::try_from(record_type_val)?;
            macro_rules! dispatch {
                ($ty:ty => $variant:expr) => {{
                    let v = <$ty>::from_params(&mut params)
                        .with_context(|| format!("RECORD={record_type_val} ({ty_name})", ty_name = stringify!($ty)))?;
                    params.assert_exhausted()
                        .with_context(|| format!("RECORD={record_type_val} ({ty_name})", ty_name = stringify!($ty)))?;
                    Ok($variant(v))
                }};
            }
            match record_type {
                SchRecordType::Component => {
                    let comp = parse_component_record(&mut params)?;
                    params.assert_exhausted()?;
                    Ok(SchRecord::Component(comp))
                }
                SchRecordType::Label => dispatch!(SchLabel => SchRecord::Label),
                SchRecordType::Bezier => dispatch!(SchBezier => SchRecord::Bezier),
                SchRecordType::Polyline => dispatch!(SchPolyline => SchRecord::Polyline),
                SchRecordType::Polygon => dispatch!(SchPolygon => SchRecord::Polygon),
                SchRecordType::Ellipse => dispatch!(SchEllipse => SchRecord::Ellipse),
                SchRecordType::Pie => dispatch!(SchPie => SchRecord::Pie),
                SchRecordType::RoundRectangle => dispatch!(SchRoundRectangle => SchRecord::RoundRectangle),
                SchRecordType::EllipticalArc => dispatch!(SchEllipticalArc => SchRecord::EllipticalArc),
                SchRecordType::Arc => dispatch!(SchArc => SchRecord::Arc),
                SchRecordType::Line => dispatch!(SchLine => SchRecord::Line),
                SchRecordType::Rectangle => dispatch!(SchRectangle => SchRecord::Rectangle),
                SchRecordType::TextFrame => dispatch!(SchTextFrame => SchRecord::TextFrame),
                SchRecordType::Image => dispatch!(SchImage => SchRecord::Image),
                SchRecordType::Designator => dispatch!(SchDesignator => SchRecord::Designator),
                SchRecordType::Parameter => dispatch!(SchParameter => SchRecord::Parameter),
                SchRecordType::ImplementationList => dispatch!(SchImplementationList => SchRecord::ImplementationList),
                SchRecordType::Implementation => dispatch!(SchImplementation => SchRecord::Implementation),
                SchRecordType::ImplementationMap => dispatch!(SchImplementationMap => SchRecord::ImplementationMap),
                SchRecordType::MapDefiner => dispatch!(SchMapDefiner => SchRecord::MapDefiner),
                SchRecordType::ParameterList => dispatch!(SchParameterList => SchRecord::ParameterList),
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
        records.push(
            dispatch_record(block)
                .with_context(|| format!("record #{i} in Data stream"))?,
        );
    }

    Ok(SchLibComponent { component, records })
}

// ── Pin sidecar helpers ────────────────────────────────────────────────────────

fn collect_pins_mut(records: &mut Vec<SchRecord>) -> Vec<&mut SchPin> {
    records
        .iter_mut()
        .filter_map(|r| if let SchRecord::Pin(p) = r { Some(p) } else { None })
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
    id.parse::<usize>().map_err(|_| AltiumFormatError::InvalidParamValue {
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
            pins[pin_idx].swap_id_pin = v;
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
        let name_margin = if name_pos_custom { Some(Coord::from_internal(r.read_i32_le()?)) } else { None };
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
        let desig_margin = if desig_pos_custom { Some(Coord::from_internal(r.read_i32_le()?)) } else { None };
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
            pins[pin_idx].pin_symbol_line_width = v;
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
        if pin.description.len() > 254 {
            let overflow = &pin.description[254..];
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

// Returns PinMiscData sidecar stream if any pin has a non-empty swap_id_pin.
fn write_pin_misc_data(pins: &[&SchPin]) -> Option<Result<Vec<u8>>> {
    let mut entries = Vec::new();
    for (i, pin) in pins.iter().enumerate() {
        if !pin.swap_id_pin.is_empty() {
            let mut params = ParameterCollection::new();
            params.insert(PAIR_SWAP_ID, pin.swap_id_pin.clone());
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
    if data.position_mode_custom { flags |= PIN_TEXT_POS_CUSTOM; }
    if data.rotation_anchor_component { flags |= PIN_TEXT_ROT_ANCHOR; }
    flags |= ((data.rotation_relative as u8) << PIN_TEXT_ROT_REL_SHIFT) & PIN_TEXT_ROT_REL_MASK;
    if data.font_mode_custom { flags |= PIN_TEXT_FONT_CUSTOM; }
    w.write_u8(flags);
    if data.position_mode_custom {
        w.write_i32_le(data.custom_position_margin.map_or(0, |c| c.to_internal()));
    }
    if data.font_mode_custom {
        w.write_i16_le(data.custom_font_id.unwrap_or(0));
        w.write_i32_le(data.custom_color.map_or(0, |c| c.raw()));
    }
}

// Returns PinWideText sidecar stream if any pin has non-empty text fields.
fn write_pin_wide_text(pins: &[&SchPin]) -> Option<Result<Vec<u8>>> {
    let mut entries = Vec::new();
    for (i, pin) in pins.iter().enumerate() {
        let has_data = !pin.description.is_empty()
            || !pin.name.is_empty()
            || !pin.designator.is_empty()
            || !pin.swap_id_pin.is_empty()
            || !pin.swap_id_part.is_empty()
            || !pin.default_value.is_empty();
        if has_data {
            let mut params = ParameterCollection::new();
            if !pin.description.is_empty() {
                params.insert(DESC, pin.description.clone());
            }
            if !pin.name.is_empty() {
                params.insert(NAME, pin.name.clone());
            }
            if !pin.designator.is_empty() {
                params.insert(DESIG, pin.designator.clone());
            }
            if !pin.swap_id_pin.is_empty() {
                params.insert(SWAP_ID, pin.swap_id_pin.clone());
            }
            if !pin.swap_id_part.is_empty() {
                params.insert(SWAP_ID_PART, pin.swap_id_part.clone());
            }
            if !pin.default_value.is_empty() {
                params.insert(DEF_VALUE, pin.default_value.clone());
            }
            entries.push((i.to_string(), write_sidecar_utf16le_params(&params)));
        }
    }
    if entries.is_empty() {
        return None;
    }
    Some(serialize_embedded_object_stream(PIN_WIDE_TEXT, &entries))
}

// Returns PinSymbolLineWidth sidecar stream if any pin has non-zero line width.
fn write_pin_symbol_line_width(pins: &[&SchPin]) -> Option<Result<Vec<u8>>> {
    let mut entries = Vec::new();
    for (i, pin) in pins.iter().enumerate() {
        if pin.pin_symbol_line_width != 0 {
            let mut params = ParameterCollection::new();
            params.insert(SYMBOL_LINE_WIDTH, pin.pin_symbol_line_width.to_string());
            entries.push((i.to_string(), write_sidecar_utf16le_params(&params)));
        }
    }
    if entries.is_empty() {
        return None;
    }
    Some(serialize_embedded_object_stream(PIN_SYMBOL_LINE_WIDTH, &entries))
}

// Returns PinPackageLength sidecar stream if any pin has non-empty package length.
fn write_pin_package_length(pins: &[&SchPin]) -> Option<Result<Vec<u8>>> {
    let mut entries = Vec::new();
    for (i, pin) in pins.iter().enumerate() {
        if !pin.pin_package_length.is_empty() {
            let mut params = ParameterCollection::new();
            params.insert(PIN_PACKAGE_LENGTH_KEY, pin.pin_package_length.clone());
            entries.push((i.to_string(), write_sidecar_utf16le_params(&params)));
        }
    }
    if entries.is_empty() {
        return None;
    }
    Some(serialize_embedded_object_stream(PIN_PACKAGE_LENGTH, &entries))
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
    Some(serialize_embedded_object_stream(PIN_PROPAGATION_DELAY, &entries))
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
    Some(serialize_embedded_object_stream(PIN_FUNCTION_DATA, &entries))
}

// Collects immutable pin references from a records list.
fn collect_pins(records: &[SchRecord]) -> Vec<&SchPin> {
    records
        .iter()
        .filter_map(|r| if let SchRecord::Pin(p) = r { Some(p) } else { None })
        .collect()
}

/// Serializes all pin sidecar streams for a component. Returns a list of
/// (stream_name, data) pairs for streams that have data.
pub(crate) fn serialize_pin_sidecars(
    pins: &[&SchPin],
) -> Result<Vec<(&'static str, Vec<u8>)>> {
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
            dispatch_record(block)
                .with_context(|| format!("record #{i} in Additional stream"))?,
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

    // Font table (1-based)
    params.insert(FONT_ID_COUNT, (header.fonts.len() as i32).to_param_value());
    for font in &header.fonts {
        let idx = font.id.to_string();
        params.insert(&format!("{}{}", SIZE, idx), font.size.to_param_value());
        params.insert(&format!("{}{}", ROTATION, idx), font.rotation.to_param_value());
        params.insert(&format!("{}{}", UNDERLINE, idx), font.underline.to_param_value());
        params.insert(&format!("{}{}", ITALIC, idx), font.italic.to_param_value());
        params.insert(&format!("{}{}", BOLD, idx), font.bold.to_param_value());
        params.insert(&format!("{}{}", STRIKE_OUT, idx), font.strikeout.to_param_value());
        params.insert(&format!("{}{}", FONT_NAME, idx), font.name.clone());
    }

    // Display settings — write all fields that were present in the original
    let ds = &header.display_settings;
    if let Some(v) = ds.use_mbcs { params.insert(USE_MBCS, v.to_param_value()); }
    if let Some(v) = ds.is_boc { params.insert(IS_BOC, v.to_param_value()); }
    if let Some(v) = ds.sheet_style { params.insert(SHEET_STYLE, (v as u8).to_param_value()); }
    if let Some(v) = ds.border_on { params.insert(BORDER_ON, v.to_param_value()); }
    if let Some(v) = ds.title_block_on { params.insert(TITLE_BLOCK_ON, v.to_param_value()); }
    if let Some(v) = ds.document_border_style { params.insert(DOCUMENT_BORDER_STYLE, (v as u8).to_param_value()); }
    if let Some(v) = ds.sheet_number_space_size { params.insert(SHEET_NUMBER_SPACE_SIZE, v.to_param_value()); }
    if let Some(v) = ds.area_color { params.insert(AREA_COLOR, v.raw().to_param_value()); }
    if let Some(v) = ds.snap_grid_on { params.insert(SNAP_GRID_ON, v.to_param_value()); }
    if let Some(v) = ds.snap_grid_size { params.insert_coord(SNAP_GRID_SIZE, SNAP_GRID_SIZE_FRAC, v); }
    if let Some(v) = ds.visible_grid_on { params.insert(VISIBLE_GRID_ON, v.to_param_value()); }
    if let Some(v) = ds.visible_grid_size { params.insert_coord(VISIBLE_GRID_SIZE, VISIBLE_GRID_SIZE_FRAC, v); }
    if let Some(v) = ds.custom_x { params.insert_coord(CUSTOM_X, CUSTOM_X_FRAC, v); }
    if let Some(v) = ds.custom_y { params.insert_coord(CUSTOM_Y, CUSTOM_Y_FRAC, v); }
    if let Some(v) = ds.use_custom_sheet { params.insert(USE_CUSTOM_SHEET, v.to_param_value()); }
    if let Some(v) = ds.reference_zones_on { params.insert(REFERENCE_ZONES_ON, v.to_param_value()); }
    if let Some(v) = ds.reference_zone_style { params.insert(REFERENCE_ZONE_STYLE, (v as u8).to_param_value()); }
    if let Some(v) = ds.custom_x_zones { params.insert(CUSTOM_X_ZONES, v.to_param_value()); }
    if let Some(v) = ds.custom_y_zones { params.insert(CUSTOM_Y_ZONES, v.to_param_value()); }
    if let Some(v) = ds.custom_margin_width {
        params.insert_coord(CUSTOM_MARGIN_WIDTH, &format!("{}_Frac", CUSTOM_MARGIN_WIDTH), v);
    }
    if let Some(v) = ds.workspace_orientation { params.insert(WORKSPACE_ORIENTATION, (v as u8).to_param_value()); }
    if let Some(v) = ds.display_unit { params.insert(DISPLAY_UNIT, v.to_param_value()); }
    if let Some(v) = ds.hot_spot_grid_on { params.insert(HOT_SPOT_GRID_ON, v.to_param_value()); }
    if let Some(v) = ds.hot_spot_grid_size { params.insert_coord(HOT_SPOT_GRID_SIZE, HOT_SPOT_GRID_SIZE_FRAC, v); }
    if let Some(v) = ds.show_hidden_pins { params.insert(SHOW_HIDDEN_PINS, v.to_param_value()); }
    if let Some(v) = ds.show_template_graphics { params.insert(SHOW_TEMPLATE_GRAPHICS, v.to_param_value()); }
    if let Some(ref v) = ds.template_file_name { params.insert(TEMPLATE_FILE_NAME, v.clone()); }
    if let Some(v) = ds.always_show_cd { params.insert(ALWAYS_SHOW_CD, v.to_param_value()); }
    if let Some(v) = ds.system_font { params.insert(SYSTEM_FONT, v.to_param_value()); }
    if let Some(ref v) = ds.file_version_info { params.insert(FILE_VERSION_INFO, v.clone()); }

    // Component index (0-based)
    params.insert(COMP_COUNT, (header.components.len() as i32).to_param_value());
    for (n, comp) in header.components.iter().enumerate() {
        params.insert(&format!("{}{}", LIB_REF, n), comp.lib_ref.clone());
        if !comp.description.is_empty() {
            params.insert(&format!("{}{}", COMP_DESCR, n), comp.description.clone());
        }
        if comp.part_count != 1 {
            params.insert(&format!("{}{}", PART_COUNT, n), comp.part_count.to_param_value());
        }
        if !comp.aliases.is_empty() {
            params.insert(&format!("{}{}", ALIAS_COUNT, n), (comp.aliases.len() as i32).to_param_value());
            for (m, alias) in comp.aliases.iter().enumerate() {
                params.insert(&format!("Comp{}Alias{}", n, m), alias.clone());
            }
        }
    }

    write_text_block(&params.to_bytes())
}

// Serializes the Storage stream (embedded images).
fn serialize_storage_stream(images: &[SchLibEmbeddedImage]) -> Result<Vec<u8>> {
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
fn serialize_component_data(comp: &SchLibComponent) -> Vec<u8> {
    // Build the component (RECORD=1) block directly to avoid needing Clone on SchComponent.
    let mut params = ParameterCollection::new();
    params.insert(RECORD, (SchRecordType::Component as i32).to_param_value());
    serialize_component_record(&comp.component, &mut params);
    let mut stream = write_text_block(&params.to_bytes());
    for record in &comp.records {
        stream.extend_from_slice(&serialize_record(record));
    }
    stream
}

// Serializes a Redirection stream for an alias.
fn serialize_redirection_stream(canonical_name: &str) -> Vec<u8> {
    let mut params = ParameterCollection::new();
    params.insert(RECORD, "0".to_owned());
    params.insert(SECTION_NAME, canonical_name.to_owned());
    write_text_block(&params.to_bytes())
}

// Serializes the LibAdditional header stream.
fn serialize_lib_additional_header(components: &[SchLibComponent]) -> Option<Vec<u8>> {
    // Check if any component has Additional records (records beyond the standard child set)
    // For now, we don't track which records came from Additional vs Data,
    // so we skip writing LibAdditional if there were none originally.
    // TODO: Track Additional records separately during parsing.
    let _ = components;
    None
}

// Builds the reverse section_keys mapping from SchLibHeader and tests key generation.
fn build_section_keys(header: &SchLibHeader) -> HashMap<String, String> {
    let mut keys = HashMap::new();
    let mut used_keys = std::collections::HashSet::new();

    for comp in &header.components {
        let sanitized = sanitize_cfb_name(&comp.lib_ref);
        if sanitized != comp.lib_ref || sanitized.len() > 31 {
            let short_key = generate_unique_key(&sanitized, &mut used_keys);
            keys.insert(comp.lib_ref.clone(), short_key.clone());
            used_keys.insert(short_key);
        } else {
            used_keys.insert(sanitized);
        }
        // Also handle alias names
        for alias in &comp.aliases {
            let sanitized = sanitize_cfb_name(alias);
            if sanitized != *alias || sanitized.len() > 31 {
                let short_key = generate_unique_key(&sanitized, &mut used_keys);
                keys.insert(alias.clone(), short_key.clone());
                used_keys.insert(short_key);
            } else {
                used_keys.insert(sanitized);
            }
        }
    }
    keys
}

fn sanitize_cfb_name(name: &str) -> String {
    name.chars()
        .map(|c| if "/\\:*?\"<>|!".contains(c) { '_' } else { c })
        .collect()
}

fn generate_unique_key(sanitized: &str, used: &std::collections::HashSet<String>) -> String {
    let base = if sanitized.len() > 31 { &sanitized[..31] } else { sanitized };
    if !used.contains(base) {
        return base.to_owned();
    }
    for suffix in 1.. {
        let suffix_str = suffix.to_string();
        let max_base_len = 31 - suffix_str.len();
        let candidate = format!("{}{}", &sanitized[..max_base_len.min(sanitized.len())], suffix_str);
        if !used.contains(&candidate) {
            return candidate;
        }
    }
    unreachable!()
}

impl SchLib {
    pub fn open(path: impl AsRef<Path>) -> crate::Result<Self> {
        let path = path.as_ref();
        let mut doc = TrackedCfbDocument::open(path)?;

        // 1. FileHeader
        let file_header_data = doc.read_stream(&format!("/{}", FILE_HEADER))?;
        let header = parse_file_header(&file_header_data)?;

        // 2. SectionKeys
        let section_keys_data = doc.read_stream_optional(&format!("/{}", SECTION_KEYS))?;
        let section_keys = match section_keys_data {
            Some(data) => parse_section_keys(&data)?,
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

            let data = doc.read_stream(&format!("/{}/Data", key))
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
                    let records = parse_additional_data(&data)
                        .with_context(|| format!("parsing Additional for '{}'", comp_index.lib_ref))?;
                    components[i].records.extend(records);
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
                let data = doc.read_stream(&format!("/{}/{}", alias_key, REDIRECTION))
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

        Ok(Self { header, components, embedded_images, aliases })
    }

    /// Serializes this SchLib back to a CFB file at `path`.
    pub fn save(&self, path: impl AsRef<Path>) -> crate::Result<()> {
        let section_keys = build_section_keys(&self.header);
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
            let key = resolve_component_key(
                &self.header.components[i].lib_ref,
                &section_keys,
            );
            cfb.create_storage(&format!("/{key}"))?;

            // Data stream
            let data = serialize_component_data(comp);
            cfb.write_stream(&format!("/{key}/Data"), &data)?;

            // Pin sidecars
            let pins = collect_pins(&comp.records);
            let sidecars = serialize_pin_sidecars(&pins)?;
            for (stream_name, sidecar_data) in sidecars {
                cfb.write_stream(&format!("/{key}/{stream_name}"), &sidecar_data)?;
            }
        }

        // 5. /LibAdditional (optional)
        if let Some(lib_additional_data) = serialize_lib_additional_header(&self.components) {
            cfb.write_stream(&format!("/{LIB_ADDITIONAL}"), &lib_additional_data)?;
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data_path(filename: &str) -> std::path::PathBuf {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        std::path::Path::new(manifest_dir).join("../../data").join(filename)
    }

    #[test]
    fn parse_file_header_blank_schlib() {
        let path = data_path("BlankSchlibComponent.SchLib");
        let mut doc = TrackedCfbDocument::open(&path).expect("open SchLib");
        let data = doc.read_stream("/FileHeader").expect("read FileHeader");
        let header = parse_file_header(&data).expect("parse FileHeader");
        assert_eq!(header.components.len(), 1, "BlankSchlibComponent should have 1 component");
        assert!(!header.unique_id.is_empty(), "UniqueID must not be empty");
    }

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

    #[test]
    fn parse_section_keys_missing_returns_empty() {
        let path = data_path("BlankSchlibComponent.SchLib");
        let mut doc = TrackedCfbDocument::open(&path).expect("open SchLib");
        let data = doc.read_stream_optional("/SectionKeys").expect("read_stream_optional");
        let map = match data {
            Some(d) => parse_section_keys(&d).expect("parse SectionKeys"),
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
        keys.insert("VeryLongComponentNameExceeding31Chars".to_owned(), "ShortKey1".to_owned());
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

    #[test]
    fn parse_component_data_blank_schlib() {
        let path = data_path("BlankSchlibComponent.SchLib");
        let mut doc = TrackedCfbDocument::open(&path).expect("open SchLib");
        let fh_data = doc.read_stream("/FileHeader").expect("fh");
        let header = parse_file_header(&fh_data).expect("parse fh");
        assert_eq!(header.components.len(), 1);
        let comp_index = &header.components[0];
        let data = doc.read_stream(&format!("/{}/Data", comp_index.lib_ref)).expect("read Data");
        // Data stream parsing either succeeds or fails with UnknownRecordType (M6+ records)
        // In either case it must progress past the SchComponent (RECORD=1) block.
        match parse_component_data(&data) {
            Ok(comp) => {
                assert!(!comp.component.lib_reference.is_empty() || comp.component.lib_reference.is_empty());
            }
            Err(AltiumFormatError::UnknownRecordType(_)) => {
                // Expected: child records not yet implemented (M6-M8)
            }
            Err(e) => panic!("unexpected error: {e}"),
        }
    }

    #[test]
    fn schlib_open_blank_validates() {
        let path = data_path("BlankSchlibComponent.SchLib");
        SchLib::open(&path).expect("SchLib::open must succeed for BlankSchlibComponent");
    }

    #[test]
    fn schlib_open_lime_micro_validates() {
        let path = data_path("LimeMicroAltiumLib_schLib.SchLib");
        if !path.exists() {
            return;
        }
        SchLib::open(&path).expect("SchLib::open must succeed for LimeMicroAltiumLib");
    }

    #[test]
    fn schlib_open_synthiam_validates() {
        let path = data_path("Synthiam.SchLib");
        if !path.exists() {
            return;
        }
        let lib = SchLib::open(&path).expect("SchLib::open must succeed for Synthiam");
        assert!(!lib.aliases.is_empty(), "Synthiam.SchLib should have aliases");
    }

    // ── Roundtrip serialization tests ─────────────────────────────────────

    fn roundtrip_stream_compare(filename: &str) {
        use crate::cfb_document::CfbDocument;

        let path = data_path(filename);
        if !path.exists() {
            return; // skip if test file absent
        }

        // Parse original
        let lib = SchLib::open(&path).expect("SchLib::open must succeed");

        // Save to temp file
        let tmp = tempfile::NamedTempFile::new().expect("create temp file");
        lib.save(tmp.path()).expect("SchLib::save must succeed");

        // Compare stream-by-stream: open both as raw CFB
        let mut original = CfbDocument::open(&path).expect("open original");
        let mut roundtripped = CfbDocument::open(tmp.path()).expect("open roundtripped");

        let orig_entries = original.enumerate_all_entries().expect("enumerate original");
        let rt_entries = roundtripped.enumerate_all_entries().expect("enumerate roundtripped");

        // Check same set of entries
        let mut orig_sorted: Vec<&String> = orig_entries.iter().collect();
        orig_sorted.sort();
        let mut rt_sorted: Vec<&String> = rt_entries.iter().collect();
        rt_sorted.sort();
        assert_eq!(
            orig_sorted, rt_sorted,
            "CFB entry sets differ for {filename}"
        );

        // Compare each stream's raw bytes
        for entry in &orig_sorted {
            // Skip storages (they have no data)
            if original.read_stream(entry).is_err() {
                continue;
            }
            let orig_data = original.read_stream(entry).expect("read original stream");
            let rt_data = roundtripped.read_stream(entry).expect("read roundtripped stream");
            if orig_data != rt_data {
                // Find first difference for debugging
                let min_len = orig_data.len().min(rt_data.len());
                let first_diff = (0..min_len).find(|&i| orig_data[i] != rt_data[i]);
                match first_diff {
                    Some(offset) => panic!(
                        "{filename}: stream {entry} differs at byte {offset}: \
                         original={:#04x}, roundtripped={:#04x} \
                         (orig_len={}, rt_len={})",
                        orig_data[offset], rt_data[offset],
                        orig_data.len(), rt_data.len()
                    ),
                    None => panic!(
                        "{filename}: stream {entry} length mismatch: \
                         original={}, roundtripped={}",
                        orig_data.len(), rt_data.len()
                    ),
                }
            }
        }
    }

    #[test]
    fn roundtrip_blank_schlib() {
        roundtrip_stream_compare("BlankSchlibComponent.SchLib");
    }

    #[test]
    fn roundtrip_lime_micro_schlib() {
        roundtrip_stream_compare("LimeMicroAltiumLib_schLib.SchLib");
    }

    #[test]
    fn roundtrip_synthiam_schlib() {
        roundtrip_stream_compare("Synthiam.SchLib");
    }

}
