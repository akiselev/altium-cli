//! Typed PcbLib system-stream metadata and strict stream codecs.
//!
//! This module models non-footprint PcbLib streams:
//! - `/FileHeader`
//! - `/SectionKeys` (optional)
//! - `/FileVersionInfo/{Header,Data}`
//! - `/Library/*`

use std::collections::BTreeMap;

use crate::error::{AltiumError, Result};
use crate::documents::section_keys::SectionKeyList;
use crate::parameters::ParameterCollection;

use super::encoding::{SIZE_FLAG_MASK, decode_win1252, encode_win1252};

const DEFAULT_FILE_HEADER_TEXT: &str = "PCB 6.0 Binary Library File";
const DEFAULT_FILE_VERSION: f64 = 5.01;

/// Typed metadata for `/FileHeader`.
#[derive(Clone, Debug)]
pub struct PcbLibFileHeaderStreamMeta {
    /// Header text string.
    pub header_text: String,
    /// Binary file format version number.
    pub file_version: f64,
    /// 8-character file key token.
    pub key: String,
}

impl Default for PcbLibFileHeaderStreamMeta {
    fn default() -> Self {
        Self {
            header_text: DEFAULT_FILE_HEADER_TEXT.to_string(),
            file_version: DEFAULT_FILE_VERSION,
            key: "AAAAAAAA".to_string(),
        }
    }
}

impl PcbLibFileHeaderStreamMeta {
    /// Serialize the typed `/FileHeader` model into stream bytes.
    pub fn to_stream_bytes(&self) -> Result<Vec<u8>> {
        let mut out = Vec::new();

        let header_bytes = encode_win1252(&self.header_text);
        if header_bytes.len() > 255 {
            return Err(AltiumError::Parse(format!(
                "pcblib FileHeader header text too long: {} bytes",
                header_bytes.len()
            )));
        }

        let key_bytes = encode_win1252(&self.key);
        if key_bytes.len() > 255 {
            return Err(AltiumError::Parse(format!(
                "pcblib FileHeader key too long: {} bytes",
                key_bytes.len()
            )));
        }

        out.extend_from_slice(&(header_bytes.len() as u32).to_le_bytes());
        out.push(header_bytes.len() as u8);
        out.extend_from_slice(&header_bytes);
        out.extend_from_slice(&self.file_version.to_le_bytes());
        out.extend_from_slice(&(key_bytes.len() as u32).to_le_bytes());
        out.push(key_bytes.len() as u8);
        out.extend_from_slice(&key_bytes);

        Ok(out)
    }
}

/// Parse strict `/FileHeader` bytes into typed metadata.
pub(crate) fn parse_file_header_stream(data: &[u8]) -> Result<PcbLibFileHeaderStreamMeta> {
    if data.len() < 4 + 1 + 1 + 8 + 4 + 1 {
        return Err(AltiumError::Parse(
            "pcblib FileHeader stream is truncated".to_string(),
        ));
    }

    let mut pos = 0usize;

    let header_block_len =
        u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
    pos += 4;
    let header_len = data[pos] as usize;
    pos += 1;
    if header_block_len == 0 || header_block_len != header_len {
        return Err(AltiumError::Parse(format!(
            "pcblib FileHeader has invalid header length block={} len={}",
            header_block_len, header_len
        )));
    }
    if pos + header_len > data.len() {
        return Err(AltiumError::Parse(
            "pcblib FileHeader overflows while reading header text".to_string(),
        ));
    }
    let header_text = String::from_utf8_lossy(&data[pos..pos + header_len]).to_string();
    pos += header_len;

    let parse_version_and_key = |start: usize| -> Result<(f64, String)> {
        let mut p = start;
        if p + 8 > data.len() {
            return Err(AltiumError::Parse(
                "pcblib FileHeader is truncated before version".to_string(),
            ));
        }
        let file_version = f64::from_le_bytes([
            data[p],
            data[p + 1],
            data[p + 2],
            data[p + 3],
            data[p + 4],
            data[p + 5],
            data[p + 6],
            data[p + 7],
        ]);
        p += 8;

        if p + 4 + 1 > data.len() {
            return Err(AltiumError::Parse(
                "pcblib FileHeader is truncated before key length".to_string(),
            ));
        }
        let key_block_len =
            u32::from_le_bytes([data[p], data[p + 1], data[p + 2], data[p + 3]]) as usize;
        p += 4;
        let key_len = data[p] as usize;
        p += 1;
        if key_block_len == 0 || key_block_len != key_len {
            return Err(AltiumError::Parse(format!(
                "pcblib FileHeader has invalid key length block={} len={}",
                key_block_len, key_len
            )));
        }
        if p + key_len != data.len() {
            return Err(AltiumError::Parse(
                "pcblib FileHeader has unexpected trailing bytes".to_string(),
            ));
        }
        let key = String::from_utf8_lossy(&data[p..p + key_len]).to_string();
        Ok((file_version, key))
    };

    // AD26 stores version directly after the header text. Older outputs from this
    // project inserted an extra 0x0A byte; accept both shapes.
    let (file_version, key) = if pos < data.len() && data[pos] == 0x0A {
        match parse_version_and_key(pos + 1) {
            Ok(parsed) => parsed,
            Err(_) => parse_version_and_key(pos)?,
        }
    } else {
        parse_version_and_key(pos)?
    };

    Ok(PcbLibFileHeaderStreamMeta {
        header_text,
        file_version,
        key,
    })
}

/// Typed metadata for `Header` + `Data` paired streams.
#[derive(Clone, Debug, Default)]
pub struct PcbLibCountedDataStreamMeta {
    /// `Header` u32 count value.
    pub header_count: u32,
    /// Raw `Data` bytes.
    pub data: Vec<u8>,
}

/// Typed metadata for `/Library/Models`.
#[derive(Clone, Debug, Default)]
pub struct PcbLibModelsStorageMeta {
    /// `/Library/Models/Header` u32 count.
    pub header_count: u32,
    /// `/Library/Models/Data` raw bytes.
    pub data: Vec<u8>,
    /// Optional numbered model blobs (`/Library/Models/<N>`).
    pub entries: BTreeMap<u32, Vec<u8>>,
}

/// Typed metadata for `/Library/*` system streams.
#[derive(Clone, Debug)]
pub struct PcbLibLibraryStorageMeta {
    /// `/Library/Header` u32 count.
    pub header_count: u32,
    /// `/Library/Data` raw bytes.
    pub data: Vec<u8>,
    /// `/Library/EmbeddedFonts` raw bytes.
    pub embedded_fonts: Vec<u8>,
    /// `/Library/ComponentParamsTOC/{Header,Data}`.
    pub component_params_toc: PcbLibCountedDataStreamMeta,
    /// `/Library/LayerKindMapping/{Header,Data}`.
    pub layer_kind_mapping: PcbLibCountedDataStreamMeta,
    /// `/Library/Models/*`.
    pub models: PcbLibModelsStorageMeta,
    /// `/Library/ModelsNoEmbed/{Header,Data}`.
    pub models_no_embed: PcbLibCountedDataStreamMeta,
    /// `/Library/PadViaLibrary/{Header,Data}`.
    pub pad_via_library: PcbLibCountedDataStreamMeta,
    /// `/Library/Textures/{Header,Data}`.
    pub textures: PcbLibCountedDataStreamMeta,
}

impl Default for PcbLibLibraryStorageMeta {
    fn default() -> Self {
        Self {
            header_count: 1,
            data: Vec::new(),
            embedded_fonts: Vec::new(),
            component_params_toc: PcbLibCountedDataStreamMeta {
                header_count: 1,
                data: Vec::new(),
            },
            layer_kind_mapping: PcbLibCountedDataStreamMeta {
                header_count: 1,
                data: Vec::new(),
            },
            models: PcbLibModelsStorageMeta::default(),
            models_no_embed: PcbLibCountedDataStreamMeta::default(),
            pad_via_library: PcbLibCountedDataStreamMeta::default(),
            textures: PcbLibCountedDataStreamMeta::default(),
        }
    }
}

/// Parse strict 4-byte `Header` stream count.
pub(crate) fn parse_u32_header_stream(data: &[u8], stream_path: &str) -> Result<u32> {
    if data.len() != 4 {
        return Err(AltiumError::Parse(format!(
            "pcblib stream '{}' expected 4-byte header, got {} bytes",
            stream_path,
            data.len()
        )));
    }
    Ok(u32::from_le_bytes([data[0], data[1], data[2], data[3]]))
}

/// Serialize a `Header` stream u32 count.
pub(crate) fn serialize_u32_header_stream(value: u32) -> [u8; 4] {
    value.to_le_bytes()
}

fn parse_section_key_string(data: &[u8], pos: &mut usize, label: &str) -> Result<String> {
    if *pos + 4 > data.len() {
        return Err(AltiumError::Parse(format!(
            "pcblib SectionKeys truncated before {} length",
            label
        )));
    }
    let block_len =
        u32::from_le_bytes([data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3]]) as usize;
    *pos += 4;
    if block_len == 0 {
        return Err(AltiumError::Parse(format!(
            "pcblib SectionKeys has zero-length {} block",
            label
        )));
    }
    if *pos + block_len > data.len() {
        return Err(AltiumError::Parse(format!(
            "pcblib SectionKeys {} block overflows stream",
            label
        )));
    }
    let payload = &data[*pos..*pos + block_len];
    *pos += block_len;

    let text_len = payload[0] as usize;
    if text_len + 1 != payload.len() {
        return Err(AltiumError::Parse(format!(
            "pcblib SectionKeys {} block has invalid inner length",
            label
        )));
    }

    Ok(String::from_utf8_lossy(&payload[1..]).to_string())
}

fn encode_section_key_string(value: &str) -> Result<Vec<u8>> {
    let mut payload = Vec::new();
    let text = encode_win1252(value);
    if text.len() > 255 {
        return Err(AltiumError::Parse(format!(
            "pcblib SectionKeys string too long: {} bytes",
            text.len()
        )));
    }
    payload.extend_from_slice(&((text.len() + 1) as u32).to_le_bytes());
    payload.push(text.len() as u8);
    payload.extend_from_slice(&text);
    Ok(payload)
}

/// Parse typed PcbLib `/SectionKeys` mapping stream.
pub(crate) fn parse_section_keys_stream(data: &[u8]) -> Result<SectionKeyList> {
    if data.len() < 4 {
        return Err(AltiumError::Parse(
            "pcblib SectionKeys stream is truncated".to_string(),
        ));
    }

    let mut pos = 0usize;
    let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
    pos += 4;

    let mut keys = SectionKeyList::new();
    for i in 0..count {
        let name = parse_section_key_string(data, &mut pos, &format!("name[{i}]"))?;
        let key = parse_section_key_string(data, &mut pos, &format!("key[{i}]"))?;
        keys.insert_mapping(&name, &key);
    }

    if pos != data.len() {
        return Err(AltiumError::Parse(
            "pcblib SectionKeys stream has trailing bytes".to_string(),
        ));
    }

    Ok(keys)
}

/// Serialize typed PcbLib `/SectionKeys` mapping stream.
pub(crate) fn serialize_section_keys_stream(keys: &SectionKeyList) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    out.extend_from_slice(&(keys.len() as u32).to_le_bytes());
    for (name, key) in keys.iter() {
        out.extend_from_slice(&encode_section_key_string(name)?);
        out.extend_from_slice(&encode_section_key_string(key)?);
    }
    Ok(out)
}

/// Typed metadata for a footprint `WideStrings` stream.
#[derive(Clone, Debug, Default)]
pub struct PcbLibWideStringsStreamMeta {
    /// Ordered parameter blocks in the stream.
    pub entries: Vec<ParameterCollection>,
}

/// One parsed row in `PrimitiveGuids/Data`.
#[derive(Clone, Debug)]
pub struct PcbLibPrimitiveGuidEntry {
    /// First 32-bit field (format-specific tag).
    pub tag: u32,
    /// Second 32-bit field (format-specific index).
    pub index: u32,
    /// Raw 16-byte GUID payload.
    pub guid: [u8; 16],
}

/// Typed metadata for `PrimitiveGuids/{Header,Data}`.
#[derive(Clone, Debug, Default)]
pub struct PcbLibPrimitiveGuidsStreamMeta {
    pub entries: Vec<PcbLibPrimitiveGuidEntry>,
}

/// Typed metadata for parameter-block table streams with `Header` + `Data`.
#[derive(Clone, Debug, Default)]
pub struct PcbLibParamTableStreamMeta {
    /// Ordered parameter blocks from `Data`.
    pub entries: Vec<ParameterCollection>,
}

/// Typed sidecar streams present under one PcbLib footprint storage.
#[derive(Clone, Debug, Default)]
pub struct PcbLibFootprintSidecarStreamsMeta {
    pub wide_strings: Option<PcbLibWideStringsStreamMeta>,
    pub primitive_guids: Option<PcbLibPrimitiveGuidsStreamMeta>,
    pub unique_id_primitive_information: Option<PcbLibParamTableStreamMeta>,
    pub extended_primitive_information: Option<PcbLibParamTableStreamMeta>,
}

impl PcbLibFootprintSidecarStreamsMeta {
    pub fn is_empty(&self) -> bool {
        self.wide_strings.is_none()
            && self.primitive_guids.is_none()
            && self.unique_id_primitive_information.is_none()
            && self.extended_primitive_information.is_none()
    }
}

fn parse_block_stream(data: &[u8], stream_path: &str) -> Result<Vec<(u8, Vec<u8>)>> {
    let mut out = Vec::new();
    let mut off = 0usize;
    let mut index = 0usize;
    while off < data.len() {
        if off + 4 > data.len() {
            return Err(AltiumError::Parse(format!(
                "pcblib stream '{}' truncated at block header {}",
                stream_path, index
            )));
        }
        let raw = u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]]);
        let flags = ((raw >> 24) & 0xFF) as u8;
        let len = (raw & SIZE_FLAG_MASK) as usize;
        off += 4;
        if len == 0 {
            return Err(AltiumError::Parse(format!(
                "pcblib stream '{}' has zero-length block {}",
                stream_path, index
            )));
        }
        if off + len > data.len() {
            return Err(AltiumError::Parse(format!(
                "pcblib stream '{}' block {} overflows stream (len={})",
                stream_path, index, len
            )));
        }
        out.push((flags, data[off..off + len].to_vec()));
        off += len;
        index += 1;
    }
    Ok(out)
}

fn decode_param_block(
    payload: &[u8],
    stream_path: &str,
    block_index: usize,
) -> Result<ParameterCollection> {
    let text_end = payload
        .iter()
        .position(|b| *b == 0)
        .unwrap_or(payload.len());
    if payload[text_end..].iter().any(|b| *b != 0) {
        return Err(AltiumError::Parse(format!(
            "pcblib stream '{}' block {} has non-zero bytes after NUL terminator",
            stream_path, block_index
        )));
    }
    let text = decode_win1252(&payload[..text_end]);
    Ok(ParameterCollection::from_string(&text))
}

fn encode_param_blocks(entries: &[ParameterCollection], stream_path: &str) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    for (i, params) in entries.iter().enumerate() {
        let mut payload = encode_win1252(&params.to_param_string());
        payload.push(0);
        if payload.len() > SIZE_FLAG_MASK as usize {
            return Err(AltiumError::Parse(format!(
                "pcblib stream '{}' block {} too large: {} bytes",
                stream_path,
                i,
                payload.len()
            )));
        }
        let header = payload.len() as u32;
        out.extend_from_slice(&header.to_le_bytes());
        out.extend_from_slice(&payload);
    }
    Ok(out)
}

/// Parse strict typed footprint `WideStrings` stream.
pub(crate) fn parse_wide_strings_stream(
    data: &[u8],
    stream_path: &str,
) -> Result<PcbLibWideStringsStreamMeta> {
    let blocks = parse_block_stream(data, stream_path)?;
    let mut entries = Vec::with_capacity(blocks.len());
    for (i, (flags, payload)) in blocks.into_iter().enumerate() {
        if flags != 0 {
            return Err(AltiumError::Parse(format!(
                "pcblib stream '{}' block {} has unsupported flags {}",
                stream_path, i, flags
            )));
        }
        entries.push(decode_param_block(&payload, stream_path, i)?);
    }
    Ok(PcbLibWideStringsStreamMeta { entries })
}

/// Serialize typed `WideStrings` stream bytes.
pub(crate) fn serialize_wide_strings_stream(
    meta: &PcbLibWideStringsStreamMeta,
    stream_path: &str,
) -> Result<Vec<u8>> {
    encode_param_blocks(&meta.entries, stream_path)
}

/// Parse strict typed `PrimitiveGuids/{Header,Data}` streams.
pub(crate) fn parse_primitive_guids_stream(
    header_data: &[u8],
    data: &[u8],
    stream_prefix: &str,
) -> Result<PcbLibPrimitiveGuidsStreamMeta> {
    let count = parse_u32_header_stream(header_data, &format!("{stream_prefix}/Header"))? as usize;
    let expected_len = count.checked_mul(24).ok_or_else(|| {
        AltiumError::Parse(format!(
            "pcblib stream '{}/Data' entry count overflow: {}",
            stream_prefix, count
        ))
    })?;
    if data.len() != expected_len {
        return Err(AltiumError::Parse(format!(
            "pcblib stream '{}/Data' expected {} bytes for {} entries, got {}",
            stream_prefix,
            expected_len,
            count,
            data.len()
        )));
    }

    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        let off = i * 24;
        let tag = u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]]);
        let index =
            u32::from_le_bytes([data[off + 4], data[off + 5], data[off + 6], data[off + 7]]);
        let mut guid = [0u8; 16];
        guid.copy_from_slice(&data[off + 8..off + 24]);
        entries.push(PcbLibPrimitiveGuidEntry { tag, index, guid });
    }

    Ok(PcbLibPrimitiveGuidsStreamMeta { entries })
}

/// Serialize typed `PrimitiveGuids/{Header,Data}` streams.
pub(crate) fn serialize_primitive_guids_stream(
    meta: &PcbLibPrimitiveGuidsStreamMeta,
) -> Result<(Vec<u8>, Vec<u8>)> {
    let count = u32::try_from(meta.entries.len()).map_err(|_| {
        AltiumError::Parse(format!(
            "pcblib PrimitiveGuids entry count too large: {}",
            meta.entries.len()
        ))
    })?;
    let mut data = Vec::with_capacity(meta.entries.len() * 24);
    for entry in &meta.entries {
        data.extend_from_slice(&entry.tag.to_le_bytes());
        data.extend_from_slice(&entry.index.to_le_bytes());
        data.extend_from_slice(&entry.guid);
    }
    Ok((count.to_le_bytes().to_vec(), data))
}

/// Parse strict typed `Header` + param-block `Data` table streams.
pub(crate) fn parse_param_table_stream(
    header_data: &[u8],
    data: &[u8],
    stream_prefix: &str,
) -> Result<PcbLibParamTableStreamMeta> {
    let count = parse_u32_header_stream(header_data, &format!("{stream_prefix}/Header"))? as usize;
    let blocks = parse_block_stream(data, &format!("{stream_prefix}/Data"))?;
    if blocks.len() != count {
        return Err(AltiumError::Parse(format!(
            "pcblib stream '{}/Data' has {} blocks but header count is {}",
            stream_prefix,
            blocks.len(),
            count
        )));
    }

    let mut entries = Vec::with_capacity(blocks.len());
    for (i, (flags, payload)) in blocks.into_iter().enumerate() {
        if flags != 0 {
            return Err(AltiumError::Parse(format!(
                "pcblib stream '{}/Data' block {} has unsupported flags {}",
                stream_prefix, i, flags
            )));
        }
        entries.push(decode_param_block(
            &payload,
            &format!("{stream_prefix}/Data"),
            i,
        )?);
    }

    Ok(PcbLibParamTableStreamMeta { entries })
}

/// Serialize typed `Header` + param-block `Data` table streams.
pub(crate) fn serialize_param_table_stream(
    meta: &PcbLibParamTableStreamMeta,
    stream_prefix: &str,
) -> Result<(Vec<u8>, Vec<u8>)> {
    let count = u32::try_from(meta.entries.len()).map_err(|_| {
        AltiumError::Parse(format!(
            "pcblib stream '{}' entry count too large: {}",
            stream_prefix,
            meta.entries.len()
        ))
    })?;
    let data = encode_param_blocks(&meta.entries, &format!("{stream_prefix}/Data"))?;
    Ok((count.to_le_bytes().to_vec(), data))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_strings_roundtrip() {
        let mut p0 = ParameterCollection::new();
        p0.add("ENCODEDTEXT0", "65,66,67");
        let meta = PcbLibWideStringsStreamMeta { entries: vec![p0] };
        let data = serialize_wide_strings_stream(&meta, "U1/WideStrings").unwrap();
        let parsed = parse_wide_strings_stream(&data, "U1/WideStrings").unwrap();
        assert_eq!(parsed.entries.len(), 1);
        assert_eq!(
            parsed.entries[0]
                .get("ENCODEDTEXT0")
                .map(|v| v.as_str().to_string()),
            Some("65,66,67".to_string())
        );
    }

    #[test]
    fn primitive_guids_roundtrip() {
        let meta = PcbLibPrimitiveGuidsStreamMeta {
            entries: vec![
                PcbLibPrimitiveGuidEntry {
                    tag: 2,
                    index: 0,
                    guid: [1u8; 16],
                },
                PcbLibPrimitiveGuidEntry {
                    tag: 4,
                    index: 1,
                    guid: [2u8; 16],
                },
            ],
        };
        let (header, data) = serialize_primitive_guids_stream(&meta).unwrap();
        let parsed = parse_primitive_guids_stream(&header, &data, "U1/PrimitiveGuids").unwrap();
        assert_eq!(parsed.entries.len(), 2);
        assert_eq!(parsed.entries[0].tag, 2);
        assert_eq!(parsed.entries[1].index, 1);
        assert_eq!(parsed.entries[1].guid, [2u8; 16]);
    }

    #[test]
    fn param_table_count_mismatch_is_error() {
        let mut p0 = ParameterCollection::new();
        p0.add("PRIMITIVEINDEX", "0");
        let meta = PcbLibParamTableStreamMeta { entries: vec![p0] };
        let (_header, data) =
            serialize_param_table_stream(&meta, "U1/UniqueIDPrimitiveInformation").unwrap();
        let bad_header = 2u32.to_le_bytes().to_vec();
        let err = parse_param_table_stream(&bad_header, &data, "U1/UniqueIDPrimitiveInformation")
            .unwrap_err();
        assert!(format!("{err}").contains("header count"));
    }
}
