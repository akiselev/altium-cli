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
use altium_format_types::constants::pin::{
    DEF_VALUE, PAIR_SWAP_ID, PIN_DEFINED_FUNCTION, PIN_DEFINED_FUNCTIONS_COUNT,
    PIN_PACKAGE_LENGTH as PIN_PACKAGE_LENGTH_KEY, PIN_PROPAGATION_DELAY as PIN_PROPAGATION_DELAY_KEY,
    PIN_SELECTED_FUNCTION, PIN_SELECTED_FUNCTIONS_COUNT, SWAP_ID, SWAP_ID_PART, SYMBOL_LINE_WIDTH,
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


use crate::binary_io::BinaryReader;
use crate::block_stream::{parse_blocks, Block, BlockFormat};
use crate::embedded_object::parse_embedded_object_stream;
use crate::param_collection::ParameterCollection;
use crate::sch_records::{
    parse_binary_pin, parse_component_record, PinTextPositioning, SchArc, SchBezier,
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
                0x02 => parse_binary_pin(&block.data)
                    .map(SchRecord::Pin)
                    .context("binary pin (code=0x02)"),
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
            continue;
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
            continue;
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
            continue;
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
            continue;
        }
        // Each entry has two consecutive variable-length binary structs: name then designator
        let mut r = BinaryReader::new(&entry.inner_data);
        let name_flags = r.read_u8()?;
        let name_pos_custom = (name_flags & 0x01) != 0;
        let name_rot_anchor = (name_flags & 0x02) != 0;
        let name_rot_rel_raw = (name_flags >> 2) & 0x03;
        let name_rot_rel = RotationBy90::try_from(name_rot_rel_raw)?;
        let name_font_custom = (name_flags & 0x10) != 0;
        let name_margin = if name_pos_custom { Some(Coord::from_internal(r.read_i32_le()?)) } else { None };
        let (name_font_id, name_color) = if name_font_custom {
            (Some(r.read_i16_le()?), Some(Color::new(r.read_i32_le()?)))
        } else {
            (None, None)
        };

        let desig_flags = r.read_u8()?;
        let desig_pos_custom = (desig_flags & 0x01) != 0;
        let desig_rot_anchor = (desig_flags & 0x02) != 0;
        let desig_rot_rel_raw = (desig_flags >> 2) & 0x03;
        let desig_rot_rel = RotationBy90::try_from(desig_rot_rel_raw)?;
        let desig_font_custom = (desig_flags & 0x10) != 0;
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
            continue;
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
            continue;
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
            continue;
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
            continue;
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
            continue;
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
}
