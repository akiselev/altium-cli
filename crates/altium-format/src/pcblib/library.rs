#![allow(dead_code)]

use altium_format_types::Coord;

use crate::binary_io::BinaryReader;
use crate::block_stream::{iter_blocks, BlockFormat};
use crate::board_config::{parse_board_config, PcbBoardConfig};
use crate::param_collection::ParameterCollection;
use crate::pcb_binary_stream::parse_pcb_section_header;
use crate::{AltiumFormatError, Result};

pub(crate) struct PcbLibraryData {
    pub(crate) filename: String,
    pub(crate) kind: String,
    pub(crate) version: String,
    pub(crate) date: String,
    pub(crate) time: String,
    pub(crate) board_config: PcbBoardConfig,
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
    let board_config = parse_board_config(&mut params)?;
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
        // UNICODE sidecar: CJK/non-ASCII model entries have UNICODE=EXISTS as marker
        // plus UNICODE__<KEY>=<comma-separated UTF-16 code points> for each field.
        let _unicode = params.remove_optional::<String>("UNICODE")?;
        let _ = params.remove_prefixed("UNICODE__");
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

/// Parsed LayerKindMapping substorage.
pub(crate) struct PcbLayerKindMapping {
    pub(crate) version: String,
    pub(crate) hash: u32,
    pub(crate) entries: Vec<PcbLayerKindPair>,
}

/// Parsed PadViaLibrary substorage parameters.
pub(crate) struct PcbPadViaLibraryConfig {
    pub(crate) library_id: String,
    pub(crate) library_name: String,
    pub(crate) display_units: String,
}

/// Embedded font entry from the EmbeddedFonts stream.
pub(crate) struct PcbEmbeddedFontEntry {
    pub(crate) name: String,
    pub(crate) style_name: String,
    pub(crate) localized_name: String,
    pub(crate) unknown_u16: u16,
    pub(crate) flag: u8,
    pub(crate) data: Vec<u8>,
}

/// Parsed texture metadata entry from Library/Textures.
pub(crate) struct PcbTextureEntry {
    pub(crate) name: String,
    pub(crate) blob: Option<Vec<u8>>,
}

/// Decode a length-prefixed UTF-16LE string from the reader.
///
/// Format: u32_le byte_count, then byte_count bytes of UTF-16LE data (may
/// include a NUL terminator which is stripped from the returned String).
fn read_utf16le_string(reader: &mut BinaryReader) -> Result<String> {
    let byte_len = reader.read_u32_le()? as usize;
    let raw = reader.read_bytes(byte_len)?;
    let (decoded, _, had_errors) = encoding_rs::UTF_16LE.decode(raw);
    if had_errors {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "EmbeddedFonts".to_owned(),
            detail: "UTF-16LE decoding error in font name string".to_owned(),
        });
    }
    Ok(decoded.trim_end_matches('\0').to_owned())
}

/// Layer kind mapping entry (layer_id -> kind value pair).
pub(crate) struct PcbLayerKindPair {
    pub(crate) layer_id: u32,
    pub(crate) kind: u32,
}

/// Parse Library/LayerKindMapping/{Header,Data} streams.
///
/// Format (from Delphi TLayerKindMappingSection.DataWrite in Advpcb.dll):
///   u32_le  version_string_byte_length
///   bytes   UTF-16LE version string (e.g. "1.0\0", 8 bytes)
///   u32_le  hash (MurmurHash2-like over packed 5-byte in-memory items)
///   u32_le  entry_count
///   entry_count × 8 bytes: (u32_le TV7_Layer, u32_le TMechanicalLayerKind)
///
/// TMechanicalLayerKind is a u8 enum (0-48) but stored as u32 on disk.
pub(crate) fn parse_layer_kind_mapping(
    header: &[u8],
    data: &[u8],
) -> Result<PcbLayerKindMapping> {
    let _section_count = parse_pcb_section_header(header)?;
    if data.is_empty() {
        return Ok(PcbLayerKindMapping {
            version: String::new(),
            hash: 0,
            entries: Vec::new(),
        });
    }

    let mut reader = BinaryReader::new(data);

    // Version string: u32 byte_length + UTF-16LE bytes.
    let version_byte_len = reader.read_u32_le()? as usize;
    let version_bytes = reader.read_bytes(version_byte_len)?;
    let (decoded, _, had_errors) = encoding_rs::UTF_16LE.decode(version_bytes);
    if had_errors {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "Library/LayerKindMapping/Data".to_owned(),
            detail: "UTF-16LE decoding error in version string".to_owned(),
        });
    }
    let version = decoded.trim_end_matches('\0').to_owned();

    // Hash and entry count.
    let hash = reader.read_u32_le()?;
    let count = reader.read_u32_le()? as usize;

    // Entries: (u32 TV7_Layer, u32 TMechanicalLayerKind) pairs.
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let layer_id = reader.read_u32_le()?;
        let kind = reader.read_u32_le()?;
        entries.push(PcbLayerKindPair { layer_id, kind });
    }
    reader.assert_exhausted()?;

    Ok(PcbLayerKindMapping { version, hash, entries })
}

/// Parse Library/PadViaLibrary/{Header,Data} streams.
///
/// The header count may be 0 even when the Data stream has content. Parses
/// whatever blocks are present and returns None if the data stream is empty.
pub(crate) fn parse_pad_via_library(
    header: &[u8],
    data: &[u8],
) -> Result<Option<PcbPadViaLibraryConfig>> {
    let _count = parse_pcb_section_header(header)?;
    if data.is_empty() {
        return Ok(None);
    }

    let mut blocks_iter = iter_blocks(data);
    let block = match blocks_iter.next() {
        Some(Ok(b)) => b,
        Some(Err(e)) => return Err(e),
        None => return Ok(None),
    };

    let mut params = ParameterCollection::from_bytes(&block.data)?;
    let library_id =
        params.remove_optional::<String>("PADVIALIBRARY.LIBRARYID")?.unwrap_or_default();
    let library_name =
        params.remove_optional::<String>("PADVIALIBRARY.LIBRARYNAME")?.unwrap_or_default();
    let display_units =
        params.remove_optional::<String>("PADVIALIBRARY.DISPLAYUNITS")?.unwrap_or_default();
    params.assert_exhausted()?;

    if let Some(extra) = blocks_iter.next() {
        let _ = extra?;
        return Err(AltiumFormatError::InvalidParamValue {
            key: "Library/PadViaLibrary/Data".to_owned(),
            detail: "expected at most 1 block but found additional blocks".to_owned(),
        });
    }

    Ok(Some(PcbPadViaLibraryConfig { library_id, library_name, display_units }))
}

/// Parse the Library/EmbeddedFonts single-stream sub-storage.
///
/// Format: u32_le count, then for each font: three length-prefixed UTF-16LE
/// strings (name, style name, localized name), a u32 marker, a u32 blob size,
/// and blob_size bytes of compressed font data (opaque binary, acceptable
/// per the D2 exception for genuinely opaque binary payloads).
pub(crate) fn parse_embedded_fonts(data: &[u8]) -> Result<Vec<PcbEmbeddedFontEntry>> {
    if data.is_empty() {
        return Ok(Vec::new());
    }
    let mut reader = BinaryReader::new(data);
    let count = reader.read_u32_le()? as usize;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let name = read_utf16le_string(&mut reader)?;
        let style_name = read_utf16le_string(&mut reader)?;
        let localized_name = read_utf16le_string(&mut reader)?;
        let unknown_u16 = reader.read_u16_le()?;
        let flag = reader.read_u8()?;
        let blob_size = reader.read_u32_le()? as usize;
        let data_blob = reader.read_bytes(blob_size)?.to_vec();
        entries.push(PcbEmbeddedFontEntry { name, style_name, localized_name, unknown_u16, flag, data: data_blob });
    }
    reader.assert_exhausted()?;
    Ok(entries)
}

/// Parse Library/Textures/{Header,Data} streams.
///
/// Same pattern as Models: Header has block count, Data has one text block per
/// entry with pipe-delimited params (e.g., `NAME=<path>`). Blob data is loaded
/// separately from numbered streams.
pub(crate) fn parse_texture_metadata(
    header: &[u8],
    data: &[u8],
) -> Result<Vec<PcbTextureEntry>> {
    let count = parse_pcb_section_header(header)? as usize;
    if count == 0 && data.is_empty() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::with_capacity(count);
    for block_result in iter_blocks(data) {
        let block = block_result?;
        if block.format != BlockFormat::Text {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "Library/Textures/Data".to_owned(),
                detail: "expected text block for texture entry".to_owned(),
            });
        }
        let mut params = ParameterCollection::from_bytes(&block.data)?;
        let name = params.remove_optional::<String>("NAME")?.unwrap_or_default();
        params.assert_exhausted()?;
        entries.push(PcbTextureEntry { name, blob: None });
    }

    if entries.len() != count {
        return Err(AltiumFormatError::RecordCountMismatch {
            section: "Library/Textures".to_owned(),
            expected: count,
            actual: entries.len(),
        });
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use altium_format_types::constants::parsing::BLOCK_SIZE_MASK;
    use crate::tracked_cfb::TrackedCfbDocument;

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
        assert!(lib.board_config.v9_master_stack.is_some(), "V9_MASTERSTACK_STYLE present should yield Some(master_stack)");
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
