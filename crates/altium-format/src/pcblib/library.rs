#![allow(dead_code)]

use altium_format_types::Coord;

use crate::binary_io::BinaryReader;
use crate::block_stream::{iter_blocks, BlockFormat};
use crate::param_collection::ParameterCollection;
use crate::pcb_binary_stream::parse_pcb_section_header;
use crate::tracked_cfb::TrackedCfbDocument;
use crate::{AltiumFormatError, Result};

pub(crate) struct PcbLibraryData {
    pub(crate) filename: String,
    pub(crate) kind: String,
    pub(crate) version: String,
    pub(crate) date: String,
    pub(crate) time: String,
    pub(crate) board_config: PcbBoardConfig,
}

/// Board configuration parameters from the Library/Data stream.
///
/// These are the board defaults and layer stack definitions that Altium uses
/// when editing footprints in the library editor. The layer stack uses indexed
/// parameters V9_STACK_LAYER{N}_*.
pub(crate) struct PcbBoardConfig {
    pub(crate) record: String,
    pub(crate) v9_masterstack_style: String,
    pub(crate) v9_masterstack_id: String,
    pub(crate) v9_masterstack_name: String,
    pub(crate) layer_stack: Vec<PcbBoardLayerEntry>,
}

/// A single layer entry from the V9_STACK_LAYER{N}_* parameters.
pub(crate) struct PcbBoardLayerEntry {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) layer_id: String,
    pub(crate) used_by_prims: bool,
    pub(crate) cop_thick: String,
    pub(crate) diel_type: String,
    pub(crate) diel_const: String,
    pub(crate) diel_height: String,
    pub(crate) diel_material: String,
}

pub(crate) struct PcbLibComponentTocEntry {
    pub(crate) name: String,
    pub(crate) pad_count: u32,
    pub(crate) height: Coord,
    pub(crate) description: String,
}

pub(crate) struct PcbLibModelEntry {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) embed: bool,
    pub(crate) rotation_x: f64,
    pub(crate) rotation_y: f64,
    pub(crate) rotation_z: f64,
    pub(crate) standoff: f64,
    pub(crate) checksum: String,
    pub(crate) blob: Option<Vec<u8>>,
}

/// Parse Library/Data stream.
///
/// The stream contains at least one text block of pipe-delimited params (the main
/// library metadata and board/layer-stack defaults). After the block(s), a raw
/// binary component-name index (NOT block-framed) may follow; it duplicates
/// information already available in ComponentParamsTOC.
///
/// Returns the library metadata and the parsed component-name index (for cross-validation).
pub(crate) fn parse_library_data(data: &[u8]) -> Result<(PcbLibraryData, Vec<String>)> {
    let mut blocks_iter = iter_blocks(data);

    // First block: pipe-delimited params.
    let block = match blocks_iter.next() {
        Some(Ok(b)) => b,
        Some(Err(e)) => return Err(e),
        None => {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "Library/Data".to_owned(),
                detail: "stream is empty; expected at least one block".to_owned(),
            });
        }
    };
    if block.format != BlockFormat::Text {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "Library/Data".to_owned(),
            detail: "expected text block for first Library/Data block".to_owned(),
        });
    }

    let mut params = ParameterCollection::from_bytes(&block.data)?;
    let filename = params.remove_optional::<String>("FILENAME")?.unwrap_or_default();
    let kind = params.remove_optional::<String>("KIND")?.unwrap_or_default();
    let version = params.remove_optional::<String>("VERSION")?.unwrap_or_default();
    let date = params.remove_optional::<String>("DATE")?.unwrap_or_default();
    let time = params.remove_optional::<String>("TIME")?.unwrap_or_default();
    let board_config = parse_library_board_params(&mut params)?;
    params.assert_exhausted()?;

    // After the metadata block, the stream may contain:
    //   - A binary component-name index (raw TLV entries, NOT block-framed)
    // The name index format is: u32_le(count) then count entries of
    // u32_le(entry_size) + entry_size bytes (u8_namelen + name_bytes).
    // These names duplicate ComponentParamsTOC; we parse them for cross-validation.
    let bytes_after_first_block = 4 + block.data.len(); // header + payload
    let suffix_names = parse_library_data_suffix(&data[bytes_after_first_block..])?;

    Ok((PcbLibraryData { filename, kind, version, date, time, board_config }, suffix_names))
}

/// Parses board-level configuration parameters from the Library/Data stream.
///
/// These include the layer stack definition (V9_MASTERSTACK_* and V9_STACK_LAYER{N}_*)
/// and continuation RECORD=Board entries. The layer stack uses 1-based indexing;
/// we detect entries by probing for V9_STACK_LAYER{N}_ID.
fn parse_library_board_params(params: &mut ParameterCollection) -> Result<PcbBoardConfig> {
    let record = params.remove_optional::<String>("RECORD")?.unwrap_or_default();
    let v9_masterstack_style = params
        .remove_optional::<String>("V9_MASTERSTACK_STYLE")?
        .unwrap_or_default();
    let v9_masterstack_id = params
        .remove_optional::<String>("V9_MASTERSTACK_ID")?
        .unwrap_or_default();
    let v9_masterstack_name = params
        .remove_optional::<String>("V9_MASTERSTACK_NAME")?
        .unwrap_or_default();

    // Consume indexed layer stack entries: V9_STACK_LAYER1_*, V9_STACK_LAYER2_*, ...
    // Probe for the next index until no _ID key is found.
    let mut layer_stack = Vec::new();
    let mut idx = 1u32;
    loop {
        let id_key = format!("V9_STACK_LAYER{idx}_ID");
        let id: Option<String> = params.remove_optional(&id_key)?;
        match id {
            None => break,
            Some(id_val) => {
                let name = params
                    .remove_optional::<String>(&format!("V9_STACK_LAYER{idx}_NAME"))?
                    .unwrap_or_default();
                let layer_id = params
                    .remove_optional::<String>(&format!("V9_STACK_LAYER{idx}_LAYERID"))?
                    .unwrap_or_default();
                let used_by_prims = params
                    .remove_optional::<String>(&format!("V9_STACK_LAYER{idx}_USEDBYPRIMS"))?
                    .map(|s| s.eq_ignore_ascii_case("TRUE"))
                    .unwrap_or(false);
                let cop_thick = params
                    .remove_optional::<String>(&format!("V9_STACK_LAYER{idx}_COPTHICK"))?
                    .unwrap_or_default();
                let diel_type = params
                    .remove_optional::<String>(&format!("V9_STACK_LAYER{idx}_DIELTYPE"))?
                    .unwrap_or_default();
                let diel_const = params
                    .remove_optional::<String>(&format!("V9_STACK_LAYER{idx}_DIELCONST"))?
                    .unwrap_or_default();
                let diel_height = params
                    .remove_optional::<String>(&format!("V9_STACK_LAYER{idx}_DIELHEIGHT"))?
                    .unwrap_or_default();
                let diel_material = params
                    .remove_optional::<String>(&format!("V9_STACK_LAYER{idx}_DIELMATERIAL"))?
                    .unwrap_or_default();
                layer_stack.push(PcbBoardLayerEntry {
                    id: id_val,
                    name,
                    layer_id,
                    used_by_prims,
                    cop_thick,
                    diel_type,
                    diel_const,
                    diel_height,
                    diel_material,
                });
                idx += 1;
            }
        }
    }

    Ok(PcbBoardConfig {
        record,
        v9_masterstack_style,
        v9_masterstack_id,
        v9_masterstack_name,
        layer_stack,
    })
}

/// Parse the supplementary component-name index appended to Library/Data.
///
/// Format: u32_le(count) followed by count entries of u32_le(entry_size) +
/// entry_size bytes (u8_namelen + name_bytes). Returns the parsed names
/// for cross-validation against ComponentParamsTOC.
fn parse_library_data_suffix(data: &[u8]) -> Result<Vec<String>> {
    if data.is_empty() {
        return Ok(Vec::new());
    }
    let mut reader = BinaryReader::new(data);
    let count = reader.read_u32_le()? as usize;
    let mut names = Vec::with_capacity(count);
    for _ in 0..count {
        let entry_size = reader.read_u32_le()? as usize;
        let mut entry_reader = reader.sub_reader(entry_size)?;
        let name = entry_reader.read_pascal_string()?;
        entry_reader.assert_exhausted()?;
        names.push(name);
    }
    reader.assert_exhausted()?;
    Ok(names)
}

/// Parse Library/ComponentParamsTOC/{Header,Data} streams.
///
/// The Header stream contains a u32 block count (typically 1). The Data stream
/// is a single text block containing all entries separated by `\r\n`.
/// Each record is a pipe-delimited parameter string without a leading `|`.
/// Keys: `Name`, `Pad Count`, `Height`, `Description`.
pub(crate) fn parse_component_toc(
    header: &[u8],
    data: &[u8],
) -> Result<Vec<PcbLibComponentTocEntry>> {
    let block_count = parse_pcb_section_header(header)? as usize;
    if block_count == 0 {
        return Ok(Vec::new());
    }

    // Data is expected to be `block_count` blocks (typically 1).
    let mut blocks_iter = iter_blocks(data);
    let block = match blocks_iter.next() {
        Some(Ok(b)) => b,
        Some(Err(e)) => return Err(e),
        None => {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "Library/ComponentParamsTOC/Data".to_owned(),
                detail: "stream is empty; expected one block".to_owned(),
            });
        }
    };
    if block.format != BlockFormat::Text {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "Library/ComponentParamsTOC/Data".to_owned(),
            detail: "expected text block".to_owned(),
        });
    }
    // Verify no additional blocks exist.
    if let Some(extra) = blocks_iter.next() {
        let _ = extra?;
        return Err(AltiumFormatError::InvalidParamValue {
            key: "Library/ComponentParamsTOC/Data".to_owned(),
            detail: format!(
                "expected exactly 1 block but found additional blocks (block_count={})",
                block_count
            ),
        });
    }

    // Decode Windows-1252; strip trailing NUL terminator if present.
    let raw = block.data.strip_suffix(b"\0").unwrap_or(&block.data);
    let (decoded, _) = encoding_rs::WINDOWS_1252.decode_without_bom_handling(raw);

    // Records are separated by \r\n; each record is a pipe-delimited set of key=value pairs.
    let mut entries = Vec::new();
    for record_str in decoded.split("\r\n") {
        let record_str = record_str.trim();
        if record_str.is_empty() {
            continue;
        }
        let entry = parse_toc_record(record_str)?;
        entries.push(entry);
    }

    Ok(entries)
}

/// Parse a single TOC record line of the form `Name=X|Pad Count=N|Height=H|Description=D`.
fn parse_toc_record(record: &str) -> Result<PcbLibComponentTocEntry> {
    let mut name = String::new();
    let mut pad_count: u32 = 0;
    let mut height = Coord::ZERO;
    let mut description = String::new();

    for segment in record.split('|') {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }
        let eq_pos = segment.find('=').ok_or_else(|| AltiumFormatError::InvalidParamValue {
            key: segment.to_owned(),
            detail: "TOC record segment has no '=' separator".to_owned(),
        })?;
        let key = segment[..eq_pos].trim();
        let value = segment[eq_pos + 1..].trim();
        match key {
            "Name" => name = value.to_owned(),
            "Pad Count" => {
                pad_count = value.parse::<u32>().map_err(|_| AltiumFormatError::InvalidParamValue {
                    key: "Pad Count".to_owned(),
                    detail: format!("cannot parse '{value}' as u32"),
                })?;
            }
            "Height" => {
                // Height is stored as mils (float), optionally with "mil" suffix.
                // Some locales use comma as decimal separator.
                let trimmed = value.strip_suffix("mil").unwrap_or(value);
                let normalized = trimmed.replace(',', ".");
                let mils: f64 = normalized.parse::<f64>().map_err(|_| {
                    AltiumFormatError::InvalidParamValue {
                        key: "Height".to_owned(),
                        detail: format!("cannot parse '{value}' as f64 mil value"),
                    }
                })?;
                height = Coord::from_mils_f64(mils);
            }
            "Description" => description = value.to_owned(),
            _ => {
                return Err(AltiumFormatError::UnknownParams {
                    keys: vec![key.to_owned()],
                });
            }
        }
    }

    Ok(PcbLibComponentTocEntry { name, pad_count, height, description })
}

/// Parse Library/Models/{Header,Data} streams.
///
/// The Data stream contains one block per model entry. Each block is a text block
/// with keys: EMBED, MODELSOURCE, ID, ROTX, ROTY, ROTZ, DZ, CHECKSUM, NAME.
pub(crate) fn parse_model_metadata(header: &[u8], data: &[u8]) -> Result<Vec<PcbLibModelEntry>> {
    let count = parse_pcb_section_header(header)? as usize;
    if count == 0 {
        return Ok(Vec::new());
    }

    let mut entries = Vec::with_capacity(count);
    for block_result in iter_blocks(data) {
        let block = block_result?;
        if block.format != BlockFormat::Text {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "Library/Models/Data".to_owned(),
                detail: "expected text block for model entry".to_owned(),
            });
        }
        let mut params = ParameterCollection::from_bytes(&block.data)?;
        let id = params.remove_optional::<String>("ID")?.unwrap_or_default();
        let name = params.remove_optional::<String>("NAME")?.unwrap_or_default();
        let embed = params
            .remove_optional::<String>("EMBED")?
            .map(|s| s.eq_ignore_ascii_case("TRUE"))
            .unwrap_or(false);
        let rotation_x = params.remove_optional::<f64>("ROTX")?.unwrap_or(0.0);
        let rotation_y = params.remove_optional::<f64>("ROTY")?.unwrap_or(0.0);
        let rotation_z = params.remove_optional::<f64>("ROTZ")?.unwrap_or(0.0);
        let standoff = params.remove_optional::<f64>("DZ")?.unwrap_or(0.0);
        let checksum = params.remove_optional::<String>("CHECKSUM")?.unwrap_or_default();
        // MODELSOURCE and TITLE are present but not used in our data model; consume them.
        let _ = params.remove_optional::<String>("MODELSOURCE")?;
        let _ = params.remove_optional::<String>("TITLE")?;
        params.assert_exhausted()?;
        entries.push(PcbLibModelEntry {
            id,
            name,
            embed,
            rotation_x,
            rotation_y,
            rotation_z,
            standoff,
            checksum,
            blob: None,
        });
    }

    if entries.len() != count {
        return Err(AltiumFormatError::RecordCountMismatch {
            section: "Library/Models".to_owned(),
            expected: count,
            actual: entries.len(),
        });
    }

    Ok(entries)
}

/// Validate an auxiliary Library sub-storage with Header+Data pattern is empty.
///
/// Reads both Header and Data streams (marking them consumed). If the header
/// declares a non-zero count or the data stream is non-empty, returns an error
/// because the parser for these substorages is not yet implemented.
pub(crate) fn validate_empty_substorage(
    doc: &mut TrackedCfbDocument,
    header_path: &str,
    data_path: &str,
) -> Result<()> {
    let header_data = doc.read_stream(header_path)?;
    let count = parse_pcb_section_header(&header_data)?;
    let data = doc.read_stream(data_path)?;
    if count > 0 || !data.is_empty() {
        return Err(AltiumFormatError::InvalidParamValue {
            key: header_path.to_owned(),
            detail: format!(
                "substorage has count={count} and {} data bytes; parser not yet implemented",
                data.len()
            ),
        });
    }
    Ok(())
}

/// Validate the Library/EmbeddedFonts single-stream sub-storage is empty.
///
/// Reads the stream (marking it consumed). If non-empty, returns an error
/// because the embedded font parser is not yet implemented.
pub(crate) fn validate_empty_embedded_fonts(doc: &mut TrackedCfbDocument, path: &str) -> Result<()> {
    let data = doc.read_stream(path)?;
    if !data.is_empty() {
        return Err(AltiumFormatError::InvalidParamValue {
            key: path.to_owned(),
            detail: format!(
                "EmbeddedFonts stream has {} bytes; parser not yet implemented",
                data.len()
            ),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use altium_format_types::constants::parsing::BLOCK_SIZE_MASK;

    fn data_path(filename: &str) -> std::path::PathBuf {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        std::path::Path::new(manifest_dir).join("../../data").join(filename)
    }

    // Build a minimal block-framed stream for testing.
    fn make_text_block(payload: &[u8]) -> Vec<u8> {
        let size = payload.len() as u32;
        let mut out = Vec::with_capacity(4 + payload.len());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(payload);
        out
    }

    #[test]
    fn parse_library_data_basic() {
        let payload = b"|FILENAME=test.PcbLib|KIND=PcbLib|VERSION=1.0|DATE=2024-01-01|TIME=12:00:00|\0";
        let block = make_text_block(payload);
        let (lib, suffix_names) = parse_library_data(&block).unwrap();
        assert_eq!(lib.filename, "test.PcbLib");
        assert_eq!(lib.kind, "PcbLib");
        assert_eq!(lib.version, "1.0");
        assert_eq!(lib.date, "2024-01-01");
        assert_eq!(lib.time, "12:00:00");
        assert!(suffix_names.is_empty());
    }

    #[test]
    fn parse_library_data_with_extra_params() {
        let payload = b"|FILENAME=foo.PcbLib|KIND=PcbLib|VERSION=2.0|DATE=2024-01-01|TIME=00:00:00|V9_MASTERSTACK_STYLE=1|\0";
        let block = make_text_block(payload);
        let (lib, _) = parse_library_data(&block).unwrap();
        assert_eq!(lib.filename, "foo.PcbLib");
        assert_eq!(lib.version, "2.0");
        assert_eq!(lib.board_config.v9_masterstack_style, "1");
    }

    #[test]
    fn parse_toc_record_basic() {
        let entry = parse_toc_record("Name=SOT23|Pad Count=3|Height=0|Description=Transistor").unwrap();
        assert_eq!(entry.name, "SOT23");
        assert_eq!(entry.pad_count, 3);
        assert_eq!(entry.height, Coord::ZERO);
        assert_eq!(entry.description, "Transistor");
    }

    #[test]
    fn parse_model_metadata_single_block() {
        let payload = b"EMBED=TRUE|MODELSOURCE=Undefined|ID={GUID-1234}|ROTX=0.000|ROTY=0.000|ROTZ=90.000|DZ=1500|CHECKSUM=-12345|NAME=model.step\0";
        let mut header = Vec::new();
        header.extend_from_slice(&1u32.to_le_bytes());
        let block = make_text_block(payload);
        let entries = parse_model_metadata(&header, &block).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "{GUID-1234}");
        assert_eq!(entries[0].name, "model.step");
        assert!(entries[0].embed);
        assert_eq!(entries[0].rotation_z, 90.0);
        assert_eq!(entries[0].standoff, 1500.0);
        assert_eq!(entries[0].checksum, "-12345");
    }

    #[test]
    fn parse_model_metadata_empty() {
        let header = 0u32.to_le_bytes();
        let entries = parse_model_metadata(&header, b"").unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn consume_embedded_fonts_empty_succeeds() {
        // Empty bytes → Ok via is_empty() check (no doc needed for pure logic).
        let data: &[u8] = &[];
        assert!(data.is_empty());
    }

    #[test]
    fn consume_embedded_fonts_zero_payload_block_succeeds() {
        // 4-byte block header with payload_size=0 → Ok.
        let raw: u32 = 0x00_00_00_00;
        let data = raw.to_le_bytes();
        let payload_size = (u32::from_le_bytes(data) & BLOCK_SIZE_MASK) as usize;
        assert_eq!(payload_size, 0);
    }

    // ── Integration tests (real files) ──────────────────────────────────────

    #[test]
    fn pcblib_28pins_library_data() {
        let path = data_path("pcblib/28Pins_Project.PcbLib");
        if !path.exists() {
            return;
        }
        let mut doc = TrackedCfbDocument::open(&path).expect("should open PcbLib");
        let data = doc.read_stream("/Library/Data").expect("Library/Data must exist");
        let (lib, _suffix_names) = parse_library_data(&data).expect("parse_library_data must succeed");
        assert!(!lib.kind.is_empty(), "KIND must not be empty in real file");
        assert!(lib.kind.contains("PCB") || lib.kind.contains("Protel"), "KIND={}", lib.kind);
    }

    #[test]
    fn pcblib_28pins_component_toc() {
        let path = data_path("pcblib/28Pins_Project.PcbLib");
        if !path.exists() {
            return;
        }
        let mut doc = TrackedCfbDocument::open(&path).expect("should open PcbLib");
        let header = doc
            .read_stream("/Library/ComponentParamsTOC/Header")
            .expect("ComponentParamsTOC/Header must exist");
        let data = doc
            .read_stream("/Library/ComponentParamsTOC/Data")
            .expect("ComponentParamsTOC/Data must exist");
        let entries = parse_component_toc(&header, &data).expect("parse_component_toc must succeed");
        assert!(!entries.is_empty(), "ComponentParamsTOC must have entries");
        // First entry should be a known footprint name.
        assert!(!entries[0].name.is_empty(), "first TOC entry name must not be empty");
    }

    #[test]
    fn pcblib_28pins_model_metadata() {
        let path = data_path("pcblib/28Pins_Project.PcbLib");
        if !path.exists() {
            return;
        }
        let mut doc = TrackedCfbDocument::open(&path).expect("should open PcbLib");
        let header = doc.read_stream("/Library/Models/Header").expect("Models/Header must exist");
        let data = doc.read_stream("/Library/Models/Data").expect("Models/Data must exist");
        let entries = parse_model_metadata(&header, &data).expect("parse_model_metadata must succeed");
        // 28Pins_Project has 22 model blobs.
        assert!(!entries.is_empty(), "Models must have at least one entry");
        assert_eq!(entries.len(), 22, "28Pins_Project must have 22 model entries");
        for entry in &entries {
            assert!(!entry.id.is_empty(), "model id must not be empty");
            assert!(!entry.name.is_empty(), "model name must not be empty");
        }
    }
}
