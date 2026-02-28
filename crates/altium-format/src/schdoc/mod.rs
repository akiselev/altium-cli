mod dispatch;
mod fileheader;
mod types;

use std::io::Read as _;
use std::path::Path;

use altium_format_types::constants::component::DESIGNATOR;
use altium_format_types::constants::file_headers::SCH_SHEET_BINARY_HEADER_V50;
use altium_format_types::constants::pin::{
    DEF_VALUE, DESIGNATOR_CUSTOM_COLOR, DESIGNATOR_CUSTOM_FONT_ID,
    DESIGNATOR_CUSTOM_POSITION_MARGIN, NAME_CUSTOM_COLOR, NAME_CUSTOM_FONT_ID,
    NAME_CUSTOM_POSITION_MARGIN, PIN_CONGLOMERATE, PIN_CONGLOMERATE_GRAPHICALLY_LOCKED,
    PIN_CONGLOMERATE_IS_HIDDEN, PIN_CONGLOMERATE_NOT_ACCESSIBLE, PIN_CONGLOMERATE_ORIENTATION_MASK,
    PIN_CONGLOMERATE_OWNER_INDEX_ADDITIONAL_LIST, PIN_CONGLOMERATE_SHOW_DESIGNATOR,
    PIN_CONGLOMERATE_SHOW_NAME, PIN_DEFINED_FUNCTION, PIN_DEFINED_FUNCTIONS_COUNT,
    PIN_DESIGNATOR_POSITION_CONGLOMERATE, PIN_LENGTH, PIN_NAME_POSITION_CONGLOMERATE,
    PIN_PACKAGE_LENGTH, PIN_PROPAGATION_DELAY, PIN_SELECTED_FUNCTION, PIN_SELECTED_FUNCTIONS_COUNT,
    SWAP_ID_PAIR, SWAP_ID_PART, SWAP_ID_PIN, SYMBOL, SYMBOL_INNER, SYMBOL_INNER_EDGE,
    SYMBOL_LINE_WIDTH, SYMBOL_OUTER, SYMBOL_OUTER_EDGE,
};
use altium_format_types::constants::record_structure::{HEADER, RECORD, RECORD_EX, WEIGHT};
use altium_format_types::constants::record_structure::{
    OWNER_INDEX, OWNER_PART_DISPLAY_MODE, OWNER_PART_ID, UNIQUE_ID,
};
use altium_format_types::constants::sheet::{
    AREA_COLOR, BORDER_ON, CUSTOM_MARGIN_WIDTH, CUSTOM_X, CUSTOM_X_FRAC, CUSTOM_X_ZONES, CUSTOM_Y,
    CUSTOM_Y_FRAC, CUSTOM_Y_ZONES, DISPLAY_UNIT, DOCUMENT_BORDER_STYLE, FILE_VERSION_INFO,
    HOT_SPOT_GRID_ON, HOT_SPOT_GRID_SIZE, HOT_SPOT_GRID_SIZE_FRAC, IS_BOC, MINOR_VERSION,
    REFERENCE_ZONE_STYLE, REFERENCE_ZONES_ON, SHEET_NUMBER_SPACE_SIZE, SHEET_STYLE,
    SHOW_HIDDEN_PINS, SHOW_TEMPLATE_GRAPHICS, SNAP_GRID_ON, SNAP_GRID_SIZE, SNAP_GRID_SIZE_FRAC,
    SYSTEM_FONT, TEMPLATE_FILE_NAME, TITLE_BLOCK_ON, USE_CUSTOM_SHEET, USE_MBCS, VISIBLE_GRID_ON,
    VISIBLE_GRID_SIZE, VISIBLE_GRID_SIZE_FRAC, WORKSPACE_ORIENTATION,
};
use altium_format_types::constants::streams::{
    ADDITIONAL, FILE_HEADER, FILES, HARNESS_CONNECTION_POINT_CONNECTOR, OBJECT_DEFINITIONS,
    REUSE_BLOCK_INFOS, REUSE_BLOCKS, REUSE_BLOCKS_V2, STORAGE,
};
use altium_format_types::constants::text::{
    BOLD, DESCRIPTION, ITALIC, NAME, STRIKE_OUT, UNDERLINE,
};
use altium_format_types::constants::vault::{
    ITEM_REVISION_GUID, PROPS_REVISION_GUID, PROPS_VAULT_GUID, RELEASE_ITEM_GUID,
    RELEASE_VAULT_GUID, TEMPLATE_ITEM_GUID, TEMPLATE_REVISION_GUID, TEMPLATE_REVISION_HRID,
    TEMPLATE_VAULT_GUID, TEMPLATE_VAULT_HRID,
};
use altium_format_types::constants::visual::{
    COLOR, FONT_ID_COUNT, FONT_NAME, LOCATION_X, LOCATION_X_FRAC, LOCATION_Y, LOCATION_Y_FRAC,
    ROTATION, SIZE,
};
use altium_format_types::sch::SchFont;
use altium_format_types::{Color, Coord, SchDisplaySettings};

use crate::block_stream::{BlockFormat, parse_blocks, write_text_block};
use crate::cfb_document::CfbDocument;
use crate::embedded_object::{parse_embedded_object_stream, serialize_embedded_object_stream};
use crate::param_collection::ParameterCollection;
use crate::param_value::ToParamValue;
use crate::util::generate_unique_id;
use crate::sch_records::{SchPin, SchPrimitiveBase, SchRecord, SchSheet, serialize_record};
use crate::schdoc::dispatch::dispatch_record_type;
use crate::schdoc::fileheader::parse_fileheader_stream;
use crate::schdoc::types::{SchDocEmbeddedObject, SchDocHeaderMetadata};
use crate::tracked_cfb::TrackedCfbDocument;
use crate::{AltiumFormatError, Result, ResultExt};

pub use types::SchDoc;

const CFB_MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
const SCHDOC_ASCII_HEADER_PREFIX: &[u8] =
    b"|HEADER=Protel for Windows - Schematic Capture Ascii File Version";

fn preflight_check_schdoc_container(path: &Path) -> Result<()> {
    let mut file = std::fs::File::open(path)?;
    let mut head = [0u8; 128];
    let read = file.read(&mut head)?;
    let head = &head[..read];

    if head.starts_with(&CFB_MAGIC) {
        return Ok(());
    }
    if head.starts_with(b"<<<<<<< ") {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "HEADER".to_owned(),
            detail: "unsupported SchDoc input: file contains unresolved merge conflict markers"
                .to_owned(),
        });
    }
    if head.starts_with(SCHDOC_ASCII_HEADER_PREFIX) {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "HEADER".to_owned(),
            detail: "unsupported SchDoc document type: ASCII SchDoc is not supported yet (expected CFB/Binary SchDoc)"
                .to_owned(),
        });
    }

    Ok(())
}

impl SchDoc {
    pub fn new_blank_ad26() -> Self {
        let sheet = SchSheet {
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
                snap_grid_size: Some(Coord::from_mils(10)),
                visible_grid_on: Some(true),
                visible_grid_size: Some(Coord::from_mils(10)),
                hot_spot_grid_on: Some(true),
                hot_spot_grid_size: Some(Coord::from_mils(4)),
                use_custom_sheet: Some(true),
                custom_x: Some(Coord::from_mils(1500)),
                custom_y: Some(Coord::from_mils(950)),
                border_on: Some(true),
                title_block_on: Some(true),
                reference_zones_on: Some(true),
                custom_x_zones: Some(6),
                custom_y_zones: Some(4),
                custom_margin_width: Some(Coord::from_mils(20)),
                sheet_number_space_size: Some(4),
                display_unit: Some(4),
                system_font: Some(1),
                use_mbcs: Some(true),
                is_boc: Some(true),
                area_color: Some(Color::new(16_317_695)),
                ..SchDisplaySettings::default()
            },
            template_vault_guid: String::new(),
            template_item_guid: String::new(),
            template_revision_guid: String::new(),
            template_vault_hrid: String::new(),
            template_revision_hrid: String::new(),
            release_vault_guid: String::new(),
            release_item_guid: String::new(),
            item_revision_guid: String::new(),
            props_vault_guid: String::new(),
            props_revision_guid: String::new(),
        };

        Self {
            header: SchDocHeaderMetadata {
                header: SCH_SHEET_BINARY_HEADER_V50.to_owned(),
                weight: 1,
                minor_version: 0,
                unique_id: generate_unique_id(),
            },
            records: vec![SchRecord::Sheet(sheet)],
            additional_records: Vec::new(),
            embedded_objects: Vec::new(),
        }
    }

    /// Convert the internal flat record list into a structured `SchDocSheet`.
    ///
    /// This resolves the OWNERINDEX-linked flat list into a nested tree of
    /// `SheetObject` variants, including components with their pins/parameters,
    /// sheet symbols with entries, wires, buses, net labels, etc.
    pub fn sheet(&self) -> Result<crate::api::SchDocSheet> {
        crate::api::schdoc_read::sheet_from_internal(&self.records, &self.additional_records)
    }

    /// Replace the internal record list with records derived from a `SchDocSheet`.
    ///
    /// This flattens the nested tree back into OWNERINDEX-linked flat records.
    /// When possible, format-internal fields (vault GUIDs, colors, etc.) are
    /// preserved from the existing records by matching on `unique_id` or
    /// `designator`.
    pub fn update_sheet(&mut self, sheet: &crate::api::SchDocSheet) -> Result<()> {
        let (records, additional_records) = crate::api::schdoc_write::sheet_to_internal(
            sheet,
            Some(&self.records),
        )?;
        self.records = records;
        self.additional_records = additional_records;
        Ok(())
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        preflight_check_schdoc_container(path.as_ref())
            .context("detecting SchDoc container format")?;
        let mut tracked =
            TrackedCfbDocument::open(path.as_ref()).context("opening SchDoc CFB container")?;

        let (root_storages, root_streams) = tracked
            .list_entries("/")
            .context("listing root CFB entries")?;
        if !root_storages.is_empty() {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "/".to_owned(),
                detail: format!(
                    "SchDoc root must not contain storages, found: {}",
                    root_storages.join(", ")
                ),
            });
        }

        for stream in &root_streams {
            match stream.as_str() {
                FILE_HEADER | STORAGE | ADDITIONAL => {}
                OBJECT_DEFINITIONS
                | REUSE_BLOCK_INFOS
                | REUSE_BLOCKS
                | REUSE_BLOCKS_V2
                | HARNESS_CONNECTION_POINT_CONNECTOR
                | FILES => {
                    return Err(AltiumFormatError::InvalidParamValue {
                        key: stream.clone(),
                        detail: "optional SchDoc stream is present but not implemented yet"
                            .to_owned(),
                    });
                }
                _ => {
                    return Err(AltiumFormatError::InvalidParamValue {
                        key: stream.clone(),
                        detail: "unexpected top-level stream for SchDoc".to_owned(),
                    });
                }
            }
        }

        let fileheader_data = tracked
            .read_stream("/FileHeader")
            .context("reading /FileHeader")?;
        let parsed_fileheader =
            parse_fileheader_stream(&fileheader_data).context("parsing /FileHeader")?;

        let embedded_objects = if root_streams.iter().any(|s| s == STORAGE) {
            parse_storage_stream(
                &tracked
                    .read_stream("/Storage")
                    .context("reading /Storage")?,
            )
            .context("parsing /Storage")?
        } else {
            Vec::new()
        };

        let additional_records = if root_streams.iter().any(|s| s == ADDITIONAL) {
            parse_additional_stream(
                &tracked
                    .read_stream("/Additional")
                    .context("reading /Additional")?,
            )
            .context("parsing /Additional")?
        } else {
            Vec::new()
        };

        validate_invariants(
            &parsed_fileheader.records,
            &additional_records,
            &embedded_objects,
        )
        .context("validating SchDoc invariants")?;

        tracked
            .assert_all_consumed()
            .context("validating SchDoc stream consumption")?;

        Ok(Self {
            header: SchDocHeaderMetadata {
                header: parsed_fileheader.header.header,
                weight: parsed_fileheader.header.weight,
                minor_version: parsed_fileheader.header.minor_version,
                unique_id: parsed_fileheader.header.unique_id,
            },
            records: parsed_fileheader.records,
            additional_records,
            embedded_objects,
        })
    }

    pub fn validate_invariants(&self) -> Result<()> {
        validate_invariants(
            &self.records,
            &self.additional_records,
            &self.embedded_objects,
        )
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        self.validate_invariants()
            .context("validating SchDoc invariants before save")?;

        let mut cfb = CfbDocument::create().context("creating SchDoc CFB container")?;

        let fileheader = serialize_fileheader_stream(&self.header, &self.records)
            .context("serializing /FileHeader")?;
        cfb.write_stream(&format!("/{FILE_HEADER}"), &fileheader)
            .context("writing /FileHeader")?;

        let storage =
            serialize_storage_stream(&self.embedded_objects).context("serializing /Storage")?;
        cfb.write_stream(&format!("/{STORAGE}"), &storage)
            .context("writing /Storage")?;

        let additional = serialize_additional_stream(&self.additional_records)
            .context("serializing /Additional")?;
        cfb.write_stream(&format!("/{ADDITIONAL}"), &additional)
            .context("writing /Additional")?;

        cfb.save_to_file(path)
            .context("saving SchDoc CFB to file")?;
        Ok(())
    }

    /// Render the entire schematic sheet.
    pub fn render(&self, canvas: &mut dyn crate::render::AltiumCanvas) -> crate::Result<()> {
        let fonts = self
            .records
            .iter()
            .find_map(|r| {
                if let crate::sch_records::SchRecord::Sheet(s) = r {
                    Some(&s.fonts)
                } else {
                    None
                }
            })
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        for record in &self.records {
            crate::render::sch::draw_sch_record(record, canvas, fonts);
        }
        for record in &self.additional_records {
            crate::render::sch::draw_sch_record(record, canvas, fonts);
        }
        Ok(())
    }
}

fn serialize_fileheader_stream(
    header: &SchDocHeaderMetadata,
    records: &[SchRecord],
) -> Result<Vec<u8>> {
    let mut header_params = ParameterCollection::new();
    header_params.insert(HEADER, SCH_SHEET_BINARY_HEADER_V50.to_owned());
    header_params.insert(WEIGHT, (records.len() as i32).to_param_value());
    header_params.insert(MINOR_VERSION, header.minor_version.to_param_value());
    header_params.insert(UNIQUE_ID, header.unique_id.clone());

    let mut stream = write_text_block(&header_params.to_bytes());
    for (idx, record) in records.iter().enumerate() {
        let block = serialize_schdoc_record(record)
            .with_context(|| format!("serializing /FileHeader record #{idx}"))?;
        stream.extend_from_slice(&block);
    }
    Ok(stream)
}

fn serialize_additional_stream(records: &[SchRecord]) -> Result<Vec<u8>> {
    let mut header_params = ParameterCollection::new();
    header_params.insert(HEADER, SCH_SHEET_BINARY_HEADER_V50.to_owned());
    // AD26 omits Weight when Additional has no records.
    if !records.is_empty() {
        header_params.insert(WEIGHT, (records.len() as i32).to_param_value());
    }

    let mut stream = write_text_block(&header_params.to_bytes());
    for (idx, record) in records.iter().enumerate() {
        let block = serialize_schdoc_record(record)
            .with_context(|| format!("serializing /Additional record #{idx}"))?;
        stream.extend_from_slice(&block);
    }
    Ok(stream)
}

fn serialize_storage_stream(objects: &[SchDocEmbeddedObject]) -> Result<Vec<u8>> {
    if objects.is_empty() {
        let mut params = ParameterCollection::new();
        params.insert(HEADER, "Icon storage".to_owned());
        return Ok(write_text_block(&params.to_bytes()));
    }

    let entries: Vec<(String, Vec<u8>)> = objects
        .iter()
        .map(|obj| (obj.id.clone(), obj.data.clone()))
        .collect();
    serialize_embedded_object_stream("Icon storage", &entries)
}

fn serialize_schdoc_record(record: &SchRecord) -> Result<Vec<u8>> {
    match record {
        SchRecord::Sheet(sheet) => Ok(serialize_sheet_record(sheet)),
        SchRecord::Pin(pin) => Ok(serialize_text_pin_record(pin)),
        _ => serialize_record(record),
    }
}

fn serialize_sheet_record(sheet: &SchSheet) -> Vec<u8> {
    let mut params = ParameterCollection::new();
    params.insert(RECORD, "31".to_owned());
    sheet.base.to_params(&mut params);

    params.insert(FONT_ID_COUNT, (sheet.fonts.len() as i32).to_param_value());
    for font in &sheet.fonts {
        let idx = font.id.to_string();
        params.insert(&format!("{FONT_NAME}{idx}"), font.name.clone());
        params.insert(&format!("{SIZE}{idx}"), font.size.to_param_value());
        if font.rotation != 0 {
            params.insert(&format!("{ROTATION}{idx}"), font.rotation.to_param_value());
        }
        if font.bold {
            params.insert(&format!("{BOLD}{idx}"), font.bold.to_param_value());
        }
        if font.italic {
            params.insert(&format!("{ITALIC}{idx}"), font.italic.to_param_value());
        }
        if font.underline {
            params.insert(
                &format!("{UNDERLINE}{idx}"),
                font.underline.to_param_value(),
            );
        }
        if font.strikeout {
            params.insert(
                &format!("{STRIKE_OUT}{idx}"),
                font.strikeout.to_param_value(),
            );
        }
    }

    let ds = &sheet.display_settings;
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
    if let Some(v) = ds.hot_spot_grid_on {
        params.insert(HOT_SPOT_GRID_ON, v.to_param_value());
    }
    if let Some(v) = ds.hot_spot_grid_size {
        params.insert_coord(HOT_SPOT_GRID_SIZE, HOT_SPOT_GRID_SIZE_FRAC, v);
    }
    if let Some(v) = ds.sheet_style {
        params.insert(SHEET_STYLE, (v as u8).to_param_value());
    }
    if let Some(v) = ds.use_custom_sheet {
        params.insert(USE_CUSTOM_SHEET, v.to_param_value());
    }
    if let Some(v) = ds.custom_x {
        params.insert_coord(CUSTOM_X, CUSTOM_X_FRAC, v);
    }
    if let Some(v) = ds.custom_y {
        params.insert_coord(CUSTOM_Y, CUSTOM_Y_FRAC, v);
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
            &format!("{CUSTOM_MARGIN_WIDTH}_Frac"),
            v,
        );
    }
    if let Some(v) = ds.sheet_number_space_size {
        params.insert(SHEET_NUMBER_SPACE_SIZE, v.to_param_value());
    }
    if let Some(v) = ds.workspace_orientation {
        params.insert(WORKSPACE_ORIENTATION, (v as u8).to_param_value());
    }
    if let Some(v) = ds.show_hidden_pins {
        params.insert(SHOW_HIDDEN_PINS, v.to_param_value());
    }
    if let Some(v) = ds.show_template_graphics {
        params.insert(SHOW_TEMPLATE_GRAPHICS, v.to_param_value());
    }
    if let Some(v) = ds.template_file_name.as_ref() {
        params.insert(TEMPLATE_FILE_NAME, v.clone());
    }
    if let Some(v) = ds.display_unit {
        params.insert(DISPLAY_UNIT, v.to_param_value());
    }
    if let Some(v) = ds.system_font {
        params.insert(SYSTEM_FONT, v.to_param_value());
    }
    if let Some(v) = ds.use_mbcs {
        params.insert(USE_MBCS, v.to_param_value());
    }
    if let Some(v) = ds.is_boc {
        params.insert(IS_BOC, v.to_param_value());
    }
    if let Some(v) = ds.area_color {
        params.insert(AREA_COLOR, v.raw().to_param_value());
    }
    if let Some(v) = ds.file_version_info.as_ref() {
        params.insert(FILE_VERSION_INFO, v.clone());
    }

    if !sheet.template_vault_guid.is_empty() {
        params.insert(TEMPLATE_VAULT_GUID, sheet.template_vault_guid.clone());
    }
    if !sheet.template_item_guid.is_empty() {
        params.insert(TEMPLATE_ITEM_GUID, sheet.template_item_guid.clone());
    }
    if !sheet.template_revision_guid.is_empty() {
        params.insert(TEMPLATE_REVISION_GUID, sheet.template_revision_guid.clone());
    }
    if !sheet.template_vault_hrid.is_empty() {
        params.insert(TEMPLATE_VAULT_HRID, sheet.template_vault_hrid.clone());
    }
    if !sheet.template_revision_hrid.is_empty() {
        params.insert(TEMPLATE_REVISION_HRID, sheet.template_revision_hrid.clone());
    }
    if !sheet.release_vault_guid.is_empty() {
        params.insert(RELEASE_VAULT_GUID, sheet.release_vault_guid.clone());
    }
    if !sheet.release_item_guid.is_empty() {
        params.insert(RELEASE_ITEM_GUID, sheet.release_item_guid.clone());
    }
    if !sheet.item_revision_guid.is_empty() {
        params.insert(ITEM_REVISION_GUID, sheet.item_revision_guid.clone());
    }
    if !sheet.props_vault_guid.is_empty() {
        params.insert(PROPS_VAULT_GUID, sheet.props_vault_guid.clone());
    }
    if !sheet.props_revision_guid.is_empty() {
        params.insert(PROPS_REVISION_GUID, sheet.props_revision_guid.clone());
    }

    write_text_block(&params.to_bytes())
}

fn serialize_text_pin_record(pin: &SchPin) -> Vec<u8> {
    let mut params = ParameterCollection::new();
    params.insert(RECORD, "2".to_owned());
    if pin.owner_index != 0 {
        params.insert(OWNER_INDEX, pin.owner_index.to_param_value());
    }
    if pin.owner_part_id != 0 {
        params.insert(OWNER_PART_ID, pin.owner_part_id.to_param_value());
    }
    if pin.owner_part_display_mode != 0 {
        params.insert(
            OWNER_PART_DISPLAY_MODE,
            (pin.owner_part_display_mode as i32).to_param_value(),
        );
    }
    if pin.symbol_inner_edge_present {
        params.insert(
            SYMBOL_INNER_EDGE,
            (pin.symbol_inner_edge as u8).to_param_value(),
        );
    }
    if pin.symbol_outer_edge_present {
        params.insert(
            SYMBOL_OUTER_EDGE,
            (pin.symbol_outer_edge as u8).to_param_value(),
        );
    }
    if pin.symbol_inside_present {
        params.insert(SYMBOL_INNER, (pin.symbol_inside as u8).to_param_value());
    }
    if pin.symbol_outside_present {
        params.insert(SYMBOL_OUTER, (pin.symbol_outside as u8).to_param_value());
    }
    if let Some(symbol) = pin.symbol {
        params.insert(SYMBOL, (symbol as u8).to_param_value());
    }
    if !pin.description.is_empty() {
        params.insert(DESCRIPTION, pin.description.clone());
    }
    if pin.formal_type as u8 != 0 {
        params.insert(
            altium_format_types::constants::electrical::FORMAL_TYPE,
            (pin.formal_type as u8).to_param_value(),
        );
    }
    if pin.electrical as u8 != 0 {
        params.insert(
            altium_format_types::constants::electrical::ELECTRICAL,
            (pin.electrical as u8).to_param_value(),
        );
    }
    params.insert(
        PIN_CONGLOMERATE,
        encode_pin_conglomerate(pin).to_param_value(),
    );
    params.insert_coord(PIN_LENGTH, "PinLength_Frac", pin.pin_length);
    params.insert_coord(LOCATION_X, LOCATION_X_FRAC, pin.location.x);
    params.insert_coord(LOCATION_Y, LOCATION_Y_FRAC, pin.location.y);
    if pin.color.raw() != 0 {
        params.insert(COLOR, pin.color.raw().to_param_value());
    }
    if !pin.name.is_empty() {
        params.insert(NAME, pin.name.clone());
    }
    if !pin.designator.is_empty() {
        params.insert(DESIGNATOR, pin.designator.clone());
    }
    if !pin.swap_id_pin.is_empty() {
        params.insert(SWAP_ID_PIN, pin.swap_id_pin.clone());
    }
    if !pin.swap_id_part.is_empty() {
        params.insert(SWAP_ID_PART, pin.swap_id_part.clone());
    }
    if !pin.swap_id_pair.is_empty() {
        params.insert(SWAP_ID_PAIR, pin.swap_id_pair.clone());
    }
    if !pin.default_value.is_empty() {
        params.insert(DEF_VALUE, pin.default_value.clone());
    }
    if let Some(v) = pin.pin_symbol_line_width {
        params.insert(SYMBOL_LINE_WIDTH, v.to_param_value());
    }
    if !pin.spice_pin_name.is_empty() {
        params.insert("SpicePinName", pin.spice_pin_name.clone());
    }
    if !pin.hidden_net_name.is_empty() {
        params.insert("HiddenNetName", pin.hidden_net_name.clone());
    }
    if !pin.unique_id.is_empty() {
        params.insert(UNIQUE_ID, pin.unique_id.clone());
    }
    if !pin.pin_package_length.is_empty() {
        params.insert(PIN_PACKAGE_LENGTH, pin.pin_package_length.clone());
    }
    if !pin.propagation_delay.is_empty() {
        params.insert(PIN_PROPAGATION_DELAY, pin.propagation_delay.clone());
    }
    if !pin.selected_functions.is_empty() {
        params.insert(
            PIN_SELECTED_FUNCTIONS_COUNT,
            (pin.selected_functions.len() as i32).to_param_value(),
        );
        for (idx, value) in pin.selected_functions.iter().enumerate() {
            params.insert(
                &format!("{PIN_SELECTED_FUNCTION}{}", idx + 1),
                value.clone(),
            );
        }
    }
    if !pin.defined_functions.is_empty() {
        params.insert(
            PIN_DEFINED_FUNCTIONS_COUNT,
            (pin.defined_functions.len() as i32).to_param_value(),
        );
        for (idx, value) in pin.defined_functions.iter().enumerate() {
            params.insert(&format!("{PIN_DEFINED_FUNCTION}{}", idx + 1), value.clone());
        }
    }

    if let Some(ref text) = pin.name_text_data {
        let mut flags = 0u8;
        if text.position_mode_custom {
            flags |= 0x01;
        }
        if text.rotation_anchor_component {
            flags |= 0x02;
        }
        flags |= ((text.rotation_relative as u8) & 0x03) << 2;
        if text.font_mode_custom {
            flags |= 0x10;
        }
        params.insert(PIN_NAME_POSITION_CONGLOMERATE, flags.to_param_value());
        if let Some(v) = text.custom_position_margin {
            params.insert(NAME_CUSTOM_POSITION_MARGIN, v.to_param_value());
        }
        if let Some(v) = text.custom_font_id {
            params.insert(NAME_CUSTOM_FONT_ID, v.to_param_value());
        }
        if let Some(v) = text.custom_color {
            params.insert(NAME_CUSTOM_COLOR, v.raw().to_param_value());
        }
    }

    if let Some(ref text) = pin.designator_text_data {
        let mut flags = 0u8;
        if text.position_mode_custom {
            flags |= 0x01;
        }
        if text.rotation_anchor_component {
            flags |= 0x02;
        }
        flags |= ((text.rotation_relative as u8) & 0x03) << 2;
        if text.font_mode_custom {
            flags |= 0x10;
        }
        params.insert(PIN_DESIGNATOR_POSITION_CONGLOMERATE, flags.to_param_value());
        if let Some(v) = text.custom_position_margin {
            params.insert(DESIGNATOR_CUSTOM_POSITION_MARGIN, v.to_param_value());
        }
        if let Some(v) = text.custom_font_id {
            params.insert(DESIGNATOR_CUSTOM_FONT_ID, v.to_param_value());
        }
        if let Some(v) = text.custom_color {
            params.insert(DESIGNATOR_CUSTOM_COLOR, v.raw().to_param_value());
        }
    }

    write_text_block(&params.to_bytes())
}

fn encode_pin_conglomerate(pin: &SchPin) -> u8 {
    let mut out = (pin.orientation as u8) & PIN_CONGLOMERATE_ORIENTATION_MASK;
    if pin.is_hidden {
        out |= PIN_CONGLOMERATE_IS_HIDDEN;
    }
    if pin.show_name {
        out |= PIN_CONGLOMERATE_SHOW_NAME;
    }
    if pin.show_designator {
        out |= PIN_CONGLOMERATE_SHOW_DESIGNATOR;
    }
    if pin.is_not_accessible {
        out |= PIN_CONGLOMERATE_NOT_ACCESSIBLE;
    }
    if pin.graphically_locked {
        out |= PIN_CONGLOMERATE_GRAPHICALLY_LOCKED;
    }
    if pin.owner_index_additional_list {
        out |= PIN_CONGLOMERATE_OWNER_INDEX_ADDITIONAL_LIST;
    }
    out
}

fn parse_storage_stream(data: &[u8]) -> Result<Vec<SchDocEmbeddedObject>> {
    let blocks = parse_blocks(data).context("parsing /Storage block stream")?;
    let entries = parse_embedded_object_stream(&blocks).context("decoding /Storage entries")?;

    Ok(entries
        .into_iter()
        .map(|e| SchDocEmbeddedObject {
            id: e.id,
            data: e.inner_data,
        })
        .collect())
}

fn parse_additional_stream(data: &[u8]) -> Result<Vec<crate::sch_records::SchRecord>> {
    let blocks = parse_blocks(data).context("parsing /Additional block stream")?;
    if blocks.is_empty() {
        return Err(AltiumFormatError::InvalidParamValue {
            key: ADDITIONAL.to_owned(),
            detail: "stream has no blocks".to_owned(),
        });
    }
    if blocks[0].format != BlockFormat::Text {
        return Err(AltiumFormatError::InvalidParamValue {
            key: ADDITIONAL.to_owned(),
            detail: "header block must be text".to_owned(),
        });
    }

    let mut header_params =
        ParameterCollection::from_bytes(&blocks[0].data).context("parsing /Additional header")?;
    let header: String = header_params
        .remove_required(HEADER)
        .context("reading /Additional HEADER")?;
    let weight: usize = header_params.remove_with_default(WEIGHT, 0usize)?;
    header_params
        .assert_exhausted()
        .context("/Additional header has unknown parameters")?;

    let mut records = Vec::with_capacity(weight);
    for (idx, block) in blocks.iter().enumerate().skip(1) {
        if block.format != BlockFormat::Text {
            return Err(AltiumFormatError::InvalidParamValue {
                key: ADDITIONAL.to_owned(),
                detail: format!("record block #{idx} must be text"),
            });
        }

        let mut params = ParameterCollection::from_bytes(&block.data)
            .with_context(|| format!("parsing /Additional block #{idx}"))?;
        let record_raw: i32 = params
            .remove_required(RECORD)
            .with_context(|| format!("/Additional block #{idx} missing RECORD"))?;
        let record_type_val = if record_raw == 254 {
            params
                .remove_required::<i32>(RECORD_EX)
                .with_context(|| format!("/Additional block #{idx} missing RECORDEX"))?
        } else {
            record_raw
        };

        let record = dispatch_record_type(record_type_val, &mut params).with_context(|| {
            format!("dispatching /Additional block #{idx} RECORD={record_type_val}")
        })?;
        params.assert_exhausted().with_context(|| {
            format!("/Additional block #{idx} RECORD={record_type_val} has unknown parameters")
        })?;
        records.push(record);
    }

    if records.len() != weight {
        return Err(AltiumFormatError::RecordCountMismatch {
            section: format!("/Additional ({header})"),
            expected: weight,
            actual: records.len(),
        });
    }

    Ok(records)
}

fn validate_invariants(
    records: &[SchRecord],
    additional_records: &[SchRecord],
    embedded_objects: &[SchDocEmbeddedObject],
) -> Result<()> {
    for (idx, record) in records.iter().enumerate() {
        validate_owner_index(
            idx,
            record,
            records.len(),
            additional_records.len(),
            "FileHeader",
        )?;
    }
    for (idx, record) in additional_records.iter().enumerate() {
        validate_owner_index(
            idx,
            record,
            records.len(),
            additional_records.len(),
            "Additional",
        )?;
    }

    for (idx, record) in records.iter().enumerate() {
        if let SchRecord::Image(image) = record {
            if image.embed_image
                && !image.file_name.is_empty()
                && !embedded_objects.iter().any(|obj| obj.id == image.file_name)
            {
                return Err(AltiumFormatError::InvalidParamValue {
                    key: "FileName".to_owned(),
                    detail: format!(
                        "embedded SchImage record #{idx} references missing storage object {:?}",
                        image.file_name
                    ),
                });
            }
        }
    }

    Ok(())
}

fn validate_owner_index(
    idx: usize,
    record: &SchRecord,
    base_count: usize,
    additional_count: usize,
    section: &str,
) -> Result<()> {
    let (owner_index, owner_is_additional) = owner_ref(record);
    if owner_index < 0 {
        return Ok(());
    }

    let owner_index = owner_index as usize;
    if owner_is_additional {
        if owner_index >= additional_count {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "OwnerIndex".to_owned(),
                detail: format!(
                    "{section} record #{idx} points to Additional owner index {owner_index}, but Additional has only {additional_count} records"
                ),
            });
        }
    } else if owner_index >= base_count {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "OwnerIndex".to_owned(),
            detail: format!(
                "{section} record #{idx} points to base owner index {owner_index}, but FileHeader has only {base_count} records"
            ),
        });
    }

    Ok(())
}

fn owner_ref(record: &SchRecord) -> (i32, bool) {
    match record {
        SchRecord::Sheet(v) => (v.base.owner_index, false),
        SchRecord::Template(v) => (v.base.owner_index, false),
        SchRecord::Wire(v) => (v.base.owner_index, false),
        SchRecord::Bus(v) => (v.base.owner_index, false),
        SchRecord::NetLabel(v) => (v.base.owner_index, false),
        SchRecord::PowerObject(v) => (v.base.owner_index, false),
        SchRecord::Port(v) => (v.base.owner_index, false),
        SchRecord::NoConnect(v) => (v.base.owner_index, false),
        SchRecord::Junction(v) => (v.base.owner_index, false),
        SchRecord::SheetName(v) => (v.base.owner_index, false),
        SchRecord::SheetFileName(v) => (v.base.owner_index, false),
        SchRecord::SheetSymbol(v) => (v.base.owner_index, false),
        SchRecord::SheetEntry(v) => (v.base.owner_index, false),
        SchRecord::BusEntry(v) => (v.base.owner_index, false),
        SchRecord::ParameterSet(v) => (v.base.owner_index, false),
        SchRecord::Note(v) => (v.base.owner_index, false),
        SchRecord::Probe(v) => (v.base.owner_index, false),
        SchRecord::CompileMask(v) => (v.base.owner_index, false),
        SchRecord::Blanket(v) => (v.base.owner_index, false),
        SchRecord::Component(v) => (v.owner_index, false),
        SchRecord::Pin(v) => (v.owner_index, v.owner_index_additional_list),
        SchRecord::Symbol(v) => (v.base.owner_index, false),
        SchRecord::Line(v) => (v.base.owner_index, false),
        SchRecord::Rectangle(v) => (v.base.owner_index, false),
        SchRecord::RoundRectangle(v) => (v.base.owner_index, false),
        SchRecord::Arc(v) => (v.base.owner_index, false),
        SchRecord::EllipticalArc(v) => (v.base.owner_index, false),
        SchRecord::Ellipse(v) => (v.base.owner_index, false),
        SchRecord::Pie(v) => (v.base.owner_index, false),
        SchRecord::Polyline(v) => (v.base.owner_index, false),
        SchRecord::Polygon(v) => (v.base.owner_index, false),
        SchRecord::Bezier(v) => (v.base.owner_index, false),
        SchRecord::Image(v) => (v.base.owner_index, false),
        SchRecord::Label(v) => (v.base.owner_index, false),
        SchRecord::Hyperlink(v) => (v.base.owner_index, false),
        SchRecord::Designator(v) => (v.base.owner_index, false),
        SchRecord::Parameter(v) => (v.base.owner_index, false),
        SchRecord::TextFrame(v) => (v.base.owner_index, false),
        SchRecord::ImplementationList(v) => (v.base.owner_index, false),
        SchRecord::Implementation(v) => (v.base.owner_index, false),
        SchRecord::ImplementationMap(v) => (v.base.owner_index, false),
        SchRecord::MapDefiner(v) => (v.base.owner_index, false),
        SchRecord::ParameterList(v) => (v.base.owner_index, false),
        SchRecord::HarnessConnector(v) => (v.base.owner_index, false),
        SchRecord::HarnessEntry(v) => (v.base.owner_index, false),
        SchRecord::HarnessConnectorType(v) => (v.base.owner_index, false),
        SchRecord::SignalHarness(v) => (v.base.owner_index, false),
        SchRecord::HighLevelCodeSymbol(v) => (v.base.owner_index, false),
        SchRecord::HighLevelCodeEntry(v) => (v.base.owner_index, false),
        SchRecord::HighLevelCodeName(v) => (v.base.owner_index, false),
        SchRecord::HighLevelCodeFileName(v) => (v.base.owner_index, false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "proptest")]
    use proptest::prelude::*;
    #[cfg(feature = "test-fixtures")]
    use std::fs;

    #[cfg(feature = "test-fixtures")]
    fn schdoc_fixture_path(name: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/schdoc")
            .join(name)
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn open_schdoc_fixture_reaches_parser_path() {
        let path = schdoc_fixture_path(
            "myriadrf_LimeSDR-XTRX__hardware_1v0_Schematics__03_Clock_Diagram.SchDoc",
        );
        match SchDoc::open(&path) {
            Ok(_) => {}
            Err(AltiumFormatError::Io(e)) => {
                panic!("unexpected IO error while opening fixture: {e}")
            }
            Err(_) => {}
        }
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn schdoc_save_roundtrip_reopens() {
        let fixture_dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/schdoc");
        if !fixture_dir.exists() {
            return;
        }

        let mut parsed: Option<SchDoc> = None;
        let entries = match fs::read_dir(&fixture_dir) {
            Ok(v) => v,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|v| v.to_str()) != Some("SchDoc") {
                continue;
            }
            if let Ok(doc) = SchDoc::open(&path) {
                parsed = Some(doc);
                break;
            }
        }

        let Some(doc) = parsed else {
            return;
        };

        let tmp = tempfile::NamedTempFile::new().expect("create temp file");
        doc.save(tmp.path()).expect("SchDoc::save must succeed");

        let saved = SchDoc::open(tmp.path()).expect("saved SchDoc must reopen");
        saved
            .validate_invariants()
            .expect("saved SchDoc must validate");
    }

    #[test]
    fn schdoc_ascii_format_reports_unsupported_type() {
        let tmp = tempfile::NamedTempFile::new().expect("create temp file");
        std::fs::write(
            tmp.path(),
            b"|HEADER=Protel for Windows - Schematic Capture Ascii File Version 5.0|WEIGHT=1\n",
        )
        .expect("write temp schdoc");
        let err = SchDoc::open(tmp.path()).expect_err("ASCII SchDoc must be rejected");
        let msg = err.to_string();
        assert!(
            msg.contains("unsupported SchDoc document type: ASCII SchDoc"),
            "unexpected error: {msg}"
        );
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn schdoc_validate_invariants_ok_on_fixture() {
        let path = schdoc_fixture_path(
            "myriadrf_LimeSDR-XTRX__hardware_1v0_Schematics__03_Clock_Diagram.SchDoc",
        );
        let doc = SchDoc::open(path).expect("open schdoc");
        doc.validate_invariants()
            .expect("fixture should satisfy schdoc invariants");
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn schdoc_validate_invariants_detects_broken_owner_index() {
        let path = schdoc_fixture_path(
            "myriadrf_LimeSDR-XTRX__hardware_1v0_Schematics__03_Clock_Diagram.SchDoc",
        );
        let mut doc = SchDoc::open(path).expect("open schdoc");
        let len = doc.records.len() as i32;
        let first = doc.records.get_mut(0).expect("record 0");
        match first {
            SchRecord::Sheet(v) => v.base.owner_index = len + 10,
            _ => panic!("unexpected first SchDoc record type"),
        }
        let err = doc
            .validate_invariants()
            .expect_err("broken owner index must fail invariants");
        assert!(err.to_string().contains("OwnerIndex"));
    }

    #[test]
    fn schdoc_new_blank_ad26_roundtrip_validates() {
        let doc = SchDoc::new_blank_ad26();
        doc.validate_invariants()
            .expect("new blank schdoc should validate");

        let tmp = tempfile::NamedTempFile::new().expect("create temp file");
        doc.save(tmp.path()).expect("save blank schdoc");
        let reopened = SchDoc::open(tmp.path()).expect("reopen blank schdoc");
        reopened
            .validate_invariants()
            .expect("reopened blank schdoc should validate");
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    #[ignore = "mutation-test demo: should fail when run explicitly"]
    fn schdoc_broken_invariant_demo_unchecked_should_fail() {
        let path = schdoc_fixture_path(
            "myriadrf_LimeSDR-XTRX__hardware_1v0_Schematics__03_Clock_Diagram.SchDoc",
        );
        let mut doc = SchDoc::open(path).expect("open schdoc");
        let len = doc.records.len() as i32;
        let first = doc.records.get_mut(0).expect("record 0");
        match first {
            SchRecord::Sheet(v) => v.base.owner_index = len + 10,
            _ => panic!("unexpected first SchDoc record type"),
        }
        doc.validate_invariants()
            .expect("this assertion is intentionally wrong; invariant checker must fail");
    }

    #[test]
    fn schdoc_blank_sheet_api() {
        let doc = SchDoc::new_blank_ad26();
        let sheet = doc.sheet().expect("blank doc sheet() must succeed");
        assert!(!sheet.fonts.is_empty(), "blank doc should have at least one font");
        assert!(sheet.objects.is_empty(), "blank doc should have no objects");
        assert!(sheet.snap_grid_on, "snap grid should default to on");
        assert!(sheet.visible_grid_on, "visible grid should default to on");
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn schdoc_sheet_api_components_exist() {
        let fixture_dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/schdoc");
        if !fixture_dir.exists() {
            return;
        }

        let entries = match fs::read_dir(&fixture_dir) {
            Ok(v) => v,
            Err(_) => return,
        };

        let mut tested = 0;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|v| v.to_str()) != Some("SchDoc") {
                continue;
            }
            let doc = match SchDoc::open(&path) {
                Ok(d) => d,
                Err(_) => continue,
            };
            let sheet = doc.sheet().expect("sheet() should work on parsed SchDoc");

            // Query methods should work
            let comps = sheet.components();
            let wires = sheet.wires();

            // The sheet should have at least some objects
            if !sheet.objects.is_empty() {
                tested += 1;
            }

            // If there are components, verify they have designators or lib_references
            for comp in &comps {
                assert!(
                    !comp.lib_reference.is_empty(),
                    "component at {:?} in {} should have a lib_reference",
                    comp.location,
                    path.display()
                );
            }

            // Wires should have at least 2 vertices
            for wire in &wires {
                assert!(
                    wire.vertices.len() >= 2,
                    "wire in {} should have at least 2 vertices, got {}",
                    path.display(),
                    wire.vertices.len()
                );
            }
        }
        assert!(tested > 0, "should have tested at least one fixture");
    }

    #[cfg(feature = "proptest")]
    fn proptest_fixture_paths() -> Vec<std::path::PathBuf> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/schdoc");
        let mut out = Vec::new();
        let entries = fs::read_dir(dir).expect("read data/schdoc");
        for entry in entries.flatten() {
            let path = entry.path();
            let is_schdoc = path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("schdoc"))
                .unwrap_or(false);
            if is_schdoc {
                out.push(path);
            }
        }
        out.sort();
        out
    }

    #[cfg(feature = "proptest")]
    proptest! {
        #![proptest_config(ProptestConfig { cases: 16, .. ProptestConfig::default() })]

        #[test]
        fn prop_schdoc_invariants_hold_for_fixtures(idx in 0usize..4096usize) {
            let fixtures = proptest_fixture_paths();
            prop_assume!(!fixtures.is_empty());
            let path = &fixtures[idx % fixtures.len()];
            let doc = SchDoc::open(path).expect("open schdoc");
            doc.validate_invariants().expect("schdoc invariant check");
        }

        #[test]
        fn prop_schdoc_invariants_reject_broken_owner_index(idx in 0usize..4096usize) {
            let fixtures = proptest_fixture_paths();
            prop_assume!(!fixtures.is_empty());
            let path = &fixtures[idx % fixtures.len()];
            let mut doc = SchDoc::open(path).expect("open schdoc");
            let len = doc.records.len() as i32;
            let first = doc.records.get_mut(0).expect("record 0");
            match first {
                SchRecord::Sheet(v) => v.base.owner_index = len + 10,
                _ => panic!("unexpected first SchDoc record type"),
            }
            let err = doc
                .validate_invariants()
                .expect_err("broken owner index must fail");
            prop_assert!(err.to_string().contains("OwnerIndex"));
        }
    }
}
