//! Typed SchDoc stream metadata and strict stream codecs.
//!
//! This module models the non-record SchDoc streams (`/FileHeader` header
//! block, `/Additional` header block, and `/Storage` icon entries) and
//! provides strict parse/serialize helpers.

use crate::error::{AltiumError, Result};
use crate::v2::parameters::ParameterCollection;

use super::encoding::{SIZE_FLAG_MASK, decode_win1252, encode_win1252};

const DEFAULT_SCHDOC_HEADER: &str =
    "Protel for Windows - Schematic Capture Binary File Version 5.0";
const DEFAULT_STORAGE_HEADER: &str = "Icon storage";
const CFB_COMPRESSED_TAG: u8 = 0xD0;

/// Header block metadata shared by `/FileHeader` and `/Additional`.
#[derive(Clone, Debug)]
pub struct SchDocHeaderBlockMeta {
    /// `HEADER` value.
    pub header: String,
    /// Optional `Weight` value.
    pub weight: Option<usize>,
    /// Optional `MinorVersion` value.
    pub minor_version: Option<u32>,
    /// Parsed full parameter set for lossless unknown-key round-tripping.
    pub params: ParameterCollection,
}

impl SchDocHeaderBlockMeta {
    /// Build the default SchDoc header block model.
    pub fn new_default(include_minor_version: bool) -> Self {
        let mut params = ParameterCollection::new();
        params.add("HEADER", DEFAULT_SCHDOC_HEADER);
        params.add("Weight", "0");
        if include_minor_version {
            params.add("MinorVersion", "2");
        }
        Self {
            header: DEFAULT_SCHDOC_HEADER.to_string(),
            weight: Some(0),
            minor_version: include_minor_version.then_some(2),
            params,
        }
    }

    fn apply_to_params(&self, params: &mut ParameterCollection) {
        set_param_if_changed(params, "HEADER", &self.header);
        set_param_usize_if_changed(params, "Weight", self.weight);
        set_param_u32_if_changed(params, "MinorVersion", self.minor_version);
    }
}

/// Typed metadata for the SchDoc `/FileHeader` stream.
#[derive(Clone, Debug)]
pub struct SchDocFileHeaderStreamMeta {
    /// Header block metadata (block 0).
    pub header_block: SchDocHeaderBlockMeta,
}

impl Default for SchDocFileHeaderStreamMeta {
    fn default() -> Self {
        Self {
            header_block: SchDocHeaderBlockMeta::new_default(true),
        }
    }
}

impl SchDocFileHeaderStreamMeta {
    /// Serialize only the stream header block (without record blocks).
    pub fn serialize_header_block(&self, weight: usize) -> Vec<u8> {
        let mut params = self.header_block.params.clone();
        let mut header_block = self.header_block.clone();
        header_block.weight = Some(weight);
        header_block.apply_to_params(&mut params);
        encode_param_block(&params)
    }
}

/// Typed metadata for the SchDoc `/Additional` stream.
#[derive(Clone, Debug)]
pub struct SchDocAdditionalStreamMeta {
    /// Header block metadata (block 0).
    pub header_block: SchDocHeaderBlockMeta,
}

impl Default for SchDocAdditionalStreamMeta {
    fn default() -> Self {
        let mut meta = SchDocHeaderBlockMeta::new_default(false);
        meta.minor_version = None;
        meta.params.remove("MinorVersion");
        Self { header_block: meta }
    }
}

impl SchDocAdditionalStreamMeta {
    /// Serialize only the stream header block (without record blocks).
    ///
    /// If `weight_override` is `Some`, the `Weight` field is forced to that
    /// value. If it is `None`, the stored optional `Weight` is used as-is.
    pub fn serialize_header_block(&self, weight_override: Option<usize>) -> Vec<u8> {
        let mut params = self.header_block.params.clone();
        let mut header_block = self.header_block.clone();
        if let Some(weight) = weight_override {
            header_block.weight = Some(weight);
        }
        header_block.apply_to_params(&mut params);
        encode_param_block(&params)
    }
}

/// Header block metadata for `/Storage`.
#[derive(Clone, Debug)]
pub struct SchDocStorageHeaderMeta {
    /// `HEADER` value (expected: `Icon storage`).
    pub header: String,
    /// Optional `Weight` value if present.
    pub weight: Option<usize>,
    /// Parsed full parameter set for lossless unknown-key round-tripping.
    pub params: ParameterCollection,
}

impl Default for SchDocStorageHeaderMeta {
    fn default() -> Self {
        let mut params = ParameterCollection::new();
        params.add("HEADER", DEFAULT_STORAGE_HEADER);
        Self {
            header: DEFAULT_STORAGE_HEADER.to_string(),
            weight: None,
            params,
        }
    }
}

/// One compressed icon payload entry in `/Storage`.
#[derive(Clone, Debug)]
pub struct SchDocStorageEntry {
    /// Embedded identifier/path from the compressed payload envelope.
    pub id: String,
    /// Flags in the inner compressed block header.
    pub compressed_flags: u8,
    /// Raw compressed bytes from the inner block.
    pub compressed_data: Vec<u8>,
}

/// Typed metadata for the SchDoc `/Storage` stream.
#[derive(Clone, Debug, Default)]
pub struct SchDocStorageStreamMeta {
    /// Header block metadata (block 0).
    pub header_block: SchDocStorageHeaderMeta,
    /// Compressed icon entries (blocks 1..N).
    pub entries: Vec<SchDocStorageEntry>,
}

impl SchDocStorageStreamMeta {
    /// Serialize a typed `/Storage` model back into stream bytes.
    pub fn to_stream_bytes(&self) -> Vec<u8> {
        let mut params = self.header_block.params.clone();
        set_param_if_changed(&mut params, "HEADER", &self.header_block.header);
        set_param_usize_if_changed(&mut params, "Weight", self.header_block.weight);

        let mut out = encode_param_block(&params);
        for entry in &self.entries {
            let mut payload = Vec::new();
            payload.push(CFB_COMPRESSED_TAG);

            let id_bytes = encode_win1252(&entry.id);
            let id_len = id_bytes.len().min(255);
            payload.push(id_len as u8);
            payload.extend_from_slice(&id_bytes[..id_len]);

            let inner_hdr =
                ((entry.compressed_flags as u32) << 24) | (entry.compressed_data.len() as u32);
            payload.extend_from_slice(&inner_hdr.to_le_bytes());
            payload.extend_from_slice(&entry.compressed_data);

            write_block(&mut out, 0x01, &payload);
        }
        out
    }
}

/// Parsed raw block from a size-prefixed stream.
#[derive(Clone, Debug)]
pub(crate) struct SchDocRawBlock {
    pub(crate) offset: usize,
    pub(crate) flags: u8,
    pub(crate) payload: Vec<u8>,
}

/// Parse a stream into strict size-prefixed blocks.
pub(crate) fn parse_stream_blocks(data: &[u8], stream_name: &str) -> Result<Vec<SchDocRawBlock>> {
    let mut blocks = Vec::new();
    let mut off = 0usize;

    while off < data.len() {
        let block_offset = off;
        if off + 4 > data.len() {
            return Err(AltiumError::Parse(format!(
                "schdoc {} truncated at block header offset {}",
                stream_name, block_offset
            )));
        }
        let raw_header =
            u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]]);
        let flags = ((raw_header >> 24) & 0xFF) as u8;
        let len = (raw_header & SIZE_FLAG_MASK) as usize;
        off += 4;

        if len == 0 {
            return Err(AltiumError::Parse(format!(
                "schdoc {} contains zero-length block at offset {}",
                stream_name, block_offset
            )));
        }
        if off + len > data.len() {
            return Err(AltiumError::Parse(format!(
                "schdoc {} block at offset {} overflows stream (len={})",
                stream_name, block_offset, len
            )));
        }

        blocks.push(SchDocRawBlock {
            offset: block_offset,
            flags,
            payload: data[off..off + len].to_vec(),
        });
        off += len;
    }

    Ok(blocks)
}

/// Parse `/FileHeader` header metadata plus raw blocks.
pub(crate) fn parse_file_header_meta_and_blocks(
    data: &[u8],
) -> Result<(SchDocFileHeaderStreamMeta, Vec<SchDocRawBlock>)> {
    let blocks = parse_stream_blocks(data, "FileHeader")?;
    if blocks.is_empty() {
        return Err(AltiumError::Parse(
            "schdoc FileHeader stream has no blocks".to_string(),
        ));
    }
    let header_block = parse_header_block(&blocks[0], "FileHeader", true)?;
    if header_block.weight.is_none() {
        return Err(AltiumError::Parse(
            "schdoc FileHeader header block missing Weight".to_string(),
        ));
    }

    Ok((SchDocFileHeaderStreamMeta { header_block }, blocks))
}

/// Parse `/Additional` header metadata plus raw blocks.
pub(crate) fn parse_additional_meta_and_blocks(
    data: &[u8],
) -> Result<(SchDocAdditionalStreamMeta, Vec<SchDocRawBlock>)> {
    let blocks = parse_stream_blocks(data, "Additional")?;
    if blocks.is_empty() {
        return Err(AltiumError::Parse(
            "schdoc Additional stream has no blocks".to_string(),
        ));
    }
    let header_block = parse_header_block(&blocks[0], "Additional", false)?;
    Ok((SchDocAdditionalStreamMeta { header_block }, blocks))
}

/// Parse typed `/Storage` metadata and entries with strict known-shape checks.
pub(crate) fn parse_storage_meta(data: &[u8]) -> Result<SchDocStorageStreamMeta> {
    let blocks = parse_stream_blocks(data, "Storage")?;
    if blocks.is_empty() {
        return Err(AltiumError::Parse(
            "schdoc Storage stream has no blocks".to_string(),
        ));
    }

    let header_block = parse_storage_header_block(&blocks[0])?;
    let mut entries = Vec::with_capacity(blocks.len().saturating_sub(1));

    for block in &blocks[1..] {
        if block.flags != 0x01 {
            return Err(AltiumError::Parse(format!(
                "schdoc Storage block at offset {} has unsupported flags {}",
                block.offset, block.flags
            )));
        }
        if block.payload.len() < 1 + 1 + 4 {
            return Err(AltiumError::Parse(format!(
                "schdoc Storage block at offset {} too short for compressed payload",
                block.offset
            )));
        }
        if block.payload[0] != CFB_COMPRESSED_TAG {
            return Err(AltiumError::Parse(format!(
                "schdoc Storage block at offset {} missing compressed tag 0xD0",
                block.offset
            )));
        }

        let id_len = block.payload[1] as usize;
        let id_start = 2usize;
        let id_end = id_start + id_len;
        let hdr_end = id_end + 4;
        if hdr_end > block.payload.len() {
            return Err(AltiumError::Parse(format!(
                "schdoc Storage block at offset {} has invalid id length {}",
                block.offset, id_len
            )));
        }

        let id = decode_win1252(&block.payload[id_start..id_end]);
        let inner_header = u32::from_le_bytes([
            block.payload[id_end],
            block.payload[id_end + 1],
            block.payload[id_end + 2],
            block.payload[id_end + 3],
        ]);
        let inner_flags = ((inner_header >> 24) & 0xFF) as u8;
        let inner_len = (inner_header & SIZE_FLAG_MASK) as usize;
        let data_end = hdr_end + inner_len;
        if data_end != block.payload.len() {
            return Err(AltiumError::Parse(format!(
                "schdoc Storage block at offset {} has invalid compressed length {}",
                block.offset, inner_len
            )));
        }

        entries.push(SchDocStorageEntry {
            id,
            compressed_flags: inner_flags,
            compressed_data: block.payload[hdr_end..data_end].to_vec(),
        });
    }

    Ok(SchDocStorageStreamMeta {
        header_block,
        entries,
    })
}

fn parse_storage_header_block(block: &SchDocRawBlock) -> Result<SchDocStorageHeaderMeta> {
    if block.flags != 0 {
        return Err(AltiumError::Parse(format!(
            "schdoc Storage header block has unsupported flags {} at offset {}",
            block.flags, block.offset
        )));
    }
    let params = parse_param_block_payload(&block.payload, "Storage", block.offset)?;
    let header = params
        .get("HEADER")
        .map(|v| v.as_str().to_string())
        .ok_or_else(|| {
            AltiumError::Parse("schdoc Storage header block missing HEADER".to_string())
        })?;
    if !header.eq_ignore_ascii_case(DEFAULT_STORAGE_HEADER) {
        return Err(AltiumError::Parse(format!(
            "schdoc Storage header block has unsupported HEADER='{}'",
            header
        )));
    }

    Ok(SchDocStorageHeaderMeta {
        header,
        weight: parse_non_negative_usize(&params, "Weight", "Storage", block.offset)?,
        params,
    })
}

fn parse_header_block(
    block: &SchDocRawBlock,
    stream_name: &str,
    require_weight: bool,
) -> Result<SchDocHeaderBlockMeta> {
    if block.flags != 0 {
        return Err(AltiumError::Parse(format!(
            "schdoc {} header block has unsupported flags {} at offset {}",
            stream_name, block.flags, block.offset
        )));
    }

    let params = parse_param_block_payload(&block.payload, stream_name, block.offset)?;
    let header = params
        .get("HEADER")
        .map(|v| v.as_str().to_string())
        .ok_or_else(|| {
            AltiumError::Parse(format!(
                "schdoc {} header block missing HEADER",
                stream_name
            ))
        })?;

    let weight = parse_non_negative_usize(&params, "Weight", stream_name, block.offset)?;
    if require_weight && weight.is_none() {
        return Err(AltiumError::Parse(format!(
            "schdoc {} header block missing Weight",
            stream_name
        )));
    }

    let minor_version = parse_non_negative_u32(&params, "MinorVersion", stream_name, block.offset)?;

    Ok(SchDocHeaderBlockMeta {
        header,
        weight,
        minor_version,
        params,
    })
}

fn parse_param_block_payload(
    payload: &[u8],
    stream_name: &str,
    block_offset: usize,
) -> Result<ParameterCollection> {
    let text_end = payload
        .iter()
        .position(|b| *b == 0)
        .unwrap_or(payload.len());
    let text = decode_win1252(&payload[..text_end]);
    let params = ParameterCollection::from_string(&text);
    if !params.contains("HEADER") {
        return Err(AltiumError::Parse(format!(
            "schdoc {} block at offset {} is not a valid header parameter block",
            stream_name, block_offset
        )));
    }
    Ok(params)
}

fn parse_non_negative_usize(
    params: &ParameterCollection,
    key: &str,
    stream_name: &str,
    block_offset: usize,
) -> Result<Option<usize>> {
    let Some(value) = params.get(key) else {
        return Ok(None);
    };
    let parsed = value.as_int().map_err(|_| {
        AltiumError::Parse(format!(
            "schdoc {} header block at offset {} has invalid {}='{}'",
            stream_name,
            block_offset,
            key,
            value.as_str()
        ))
    })?;
    if parsed < 0 {
        return Err(AltiumError::Parse(format!(
            "schdoc {} header block at offset {} has negative {}={}",
            stream_name, block_offset, key, parsed
        )));
    }
    Ok(Some(parsed as usize))
}

fn parse_non_negative_u32(
    params: &ParameterCollection,
    key: &str,
    stream_name: &str,
    block_offset: usize,
) -> Result<Option<u32>> {
    Ok(parse_non_negative_usize(params, key, stream_name, block_offset)?.map(|v| v as u32))
}

fn set_param_if_changed(params: &mut ParameterCollection, key: &str, value: &str) {
    let needs_update = match params.get(key) {
        Some(existing) => existing.as_str() != value,
        None => true,
    };
    if needs_update {
        params.add(key, value);
    }
}

fn set_param_usize_if_changed(params: &mut ParameterCollection, key: &str, value: Option<usize>) {
    match value {
        Some(v) => {
            let current = params
                .get(key)
                .and_then(|x| x.as_int().ok())
                .filter(|x| *x >= 0)
                .map(|x| x as usize);
            if current != Some(v) {
                params.add(key, &v.to_string());
            }
        }
        None => {
            if params.contains(key) {
                params.remove(key);
            }
        }
    }
}

fn set_param_u32_if_changed(params: &mut ParameterCollection, key: &str, value: Option<u32>) {
    match value {
        Some(v) => {
            let current = params
                .get(key)
                .and_then(|x| x.as_int().ok())
                .filter(|x| *x >= 0)
                .map(|x| x as u32);
            if current != Some(v) {
                params.add(key, &v.to_string());
            }
        }
        None => {
            if params.contains(key) {
                params.remove(key);
            }
        }
    }
}

fn encode_param_block(params: &ParameterCollection) -> Vec<u8> {
    let mut payload = encode_win1252(&params.to_param_string());
    payload.push(0);
    let mut out = Vec::with_capacity(payload.len() + 4);
    write_block(&mut out, 0x00, &payload);
    out
}

fn write_block(out: &mut Vec<u8>, flags: u8, payload: &[u8]) {
    let header = ((flags as u32) << 24) | (payload.len() as u32);
    out.extend_from_slice(&header.to_le_bytes());
    out.extend_from_slice(payload);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_storage_strictly_rejects_non_icon_header() {
        let mut params = ParameterCollection::new();
        params.add("HEADER", "Something Else");
        let mut stream = encode_param_block(&params);
        let mut payload = vec![CFB_COMPRESSED_TAG, 0];
        payload.extend_from_slice(&(0u32).to_le_bytes());
        write_block(&mut stream, 0x01, &payload);
        let err = parse_storage_meta(&stream).unwrap_err();
        assert!(format!("{err}").contains("unsupported HEADER"));
    }

    #[test]
    fn storage_roundtrip_preserves_entry_count() {
        let mut storage = SchDocStorageStreamMeta::default();
        storage.entries.push(SchDocStorageEntry {
            id: "foo.bmp".to_string(),
            compressed_flags: 0,
            compressed_data: vec![1, 2, 3, 4],
        });
        let data = storage.to_stream_bytes();
        let parsed = parse_storage_meta(&data).unwrap();
        assert_eq!(parsed.entries.len(), 1);
        assert_eq!(parsed.entries[0].id, "foo.bmp");
        assert_eq!(parsed.entries[0].compressed_data, vec![1, 2, 3, 4]);
    }
}
