//! Typed SchLib non-record stream metadata and strict codecs.
//!
//! This module currently models SchLib alias redirection streams
//! (`/<AliasKey>/Redirection`) as typed parameter blocks.

use crate::error::{AltiumError, Result};
use crate::parameters::ParameterCollection;

use super::encoding::{SIZE_FLAG_MASK, decode_win1252, encode_single_param_block, encode_win1252};
use super::schdoc_streams::parse_stream_blocks;

const CFB_COMPRESSED_TAG: u8 = 0xD0;

pub(crate) const STREAM_PIN_FRAC: &str = "PinFrac";
pub(crate) const STREAM_PIN_TEXT_DATA: &str = "PinTextData";
pub(crate) const STREAM_PIN_SYMBOL_LINE_WIDTH: &str = "PinSymbolLineWidth";
pub(crate) const STREAM_PIN_PACKAGE_LENGTH: &str = "PinPackageLength";

/// Typed metadata for a SchLib alias redirection stream.
#[derive(Clone, Debug, Default)]
pub struct SchLibRedirectionStreamMeta {
    /// Destination component section name (`SECTIONNAME`).
    pub section_name: String,
    /// Full parsed parameter set for unknown-key round-tripping.
    pub params: ParameterCollection,
}

/// Parse a strict SchLib redirection stream from raw bytes.
pub(crate) fn parse_redirection_stream(
    data: &[u8],
    stream_path: &str,
) -> Result<SchLibRedirectionStreamMeta> {
    if data.len() < 4 {
        return Err(AltiumError::Parse(format!(
            "schlib redirection stream '{}' is truncated",
            stream_path
        )));
    }

    let raw_header = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let flags = ((raw_header >> 24) & 0xFF) as u8;
    let len = (raw_header & SIZE_FLAG_MASK) as usize;
    if flags != 0 {
        return Err(AltiumError::Parse(format!(
            "schlib redirection stream '{}' has unsupported flags {}",
            stream_path, flags
        )));
    }
    if len == 0 {
        return Err(AltiumError::Parse(format!(
            "schlib redirection stream '{}' has zero-length payload",
            stream_path
        )));
    }
    if 4 + len != data.len() {
        return Err(AltiumError::Parse(format!(
            "schlib redirection stream '{}' has unexpected trailing data",
            stream_path
        )));
    }

    let payload = &data[4..];
    let text_end = payload
        .iter()
        .position(|b| *b == 0)
        .unwrap_or(payload.len());
    if payload[text_end..].iter().any(|b| *b != 0) {
        return Err(AltiumError::Parse(format!(
            "schlib redirection stream '{}' contains non-zero bytes after NUL terminator",
            stream_path
        )));
    }

    let text = decode_win1252(&payload[..text_end]);
    let params = ParameterCollection::from_string(&text);
    let section_name = params
        .get("SECTIONNAME")
        .map(|v| v.as_str().to_string())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| {
            AltiumError::Parse(format!(
                "schlib redirection stream '{}' missing SECTIONNAME",
                stream_path
            ))
        })?;

    Ok(SchLibRedirectionStreamMeta {
        section_name,
        params,
    })
}

/// Serialize typed SchLib redirection metadata back to stream bytes.
pub(crate) fn serialize_redirection_stream(meta: &SchLibRedirectionStreamMeta) -> Vec<u8> {
    let mut params = meta.params.clone();
    params.add("SECTIONNAME", &meta.section_name);
    encode_single_param_block(&params)
}

/// One compressed entry in a SchLib component sidecar stream.
#[derive(Clone, Debug)]
pub struct SchLibEmbeddedObjectEntry {
    /// Embedded object identifier from the envelope.
    pub id: String,
    /// Inner compressed-block flags.
    pub compressed_flags: u8,
    /// Raw compressed bytes from the inner envelope.
    pub compressed_data: Vec<u8>,
}

/// Typed metadata for a SchLib component sidecar stream (`PinFrac`, etc.).
#[derive(Clone, Debug, Default)]
pub struct SchLibEmbeddedObjectStreamMeta {
    /// Header stream name (`HEADER` parameter).
    pub header: String,
    /// Optional `Weight` value.
    pub weight: Option<usize>,
    /// Full parsed header parameter set.
    pub params: ParameterCollection,
    /// Compressed object entries.
    pub entries: Vec<SchLibEmbeddedObjectEntry>,
}

/// Typed sidecar streams present under a SchLib component storage.
#[derive(Clone, Debug, Default)]
pub struct SchLibComponentSidecarStreamsMeta {
    pub pin_frac: Option<SchLibEmbeddedObjectStreamMeta>,
    pub pin_text_data: Option<SchLibEmbeddedObjectStreamMeta>,
    pub pin_symbol_line_width: Option<SchLibEmbeddedObjectStreamMeta>,
    pub pin_package_length: Option<SchLibEmbeddedObjectStreamMeta>,
}

impl SchLibComponentSidecarStreamsMeta {
    pub fn is_empty(&self) -> bool {
        self.pin_frac.is_none()
            && self.pin_text_data.is_none()
            && self.pin_symbol_line_width.is_none()
            && self.pin_package_length.is_none()
    }
}

/// Parse one strict SchLib component sidecar stream.
pub(crate) fn parse_component_embedded_stream(
    data: &[u8],
    stream_path: &str,
    expected_header: &str,
) -> Result<SchLibEmbeddedObjectStreamMeta> {
    let blocks = parse_stream_blocks(data, stream_path)?;
    if blocks.is_empty() {
        return Err(AltiumError::Parse(format!(
            "schlib stream '{}' has no blocks",
            stream_path
        )));
    }

    let header_block = &blocks[0];
    if header_block.flags != 0 {
        return Err(AltiumError::Parse(format!(
            "schlib stream '{}' header block has unsupported flags {}",
            stream_path, header_block.flags
        )));
    }

    let text_end = header_block
        .payload
        .iter()
        .position(|b| *b == 0)
        .unwrap_or(header_block.payload.len());
    if header_block.payload[text_end..].iter().any(|b| *b != 0) {
        return Err(AltiumError::Parse(format!(
            "schlib stream '{}' header has non-zero bytes after NUL terminator",
            stream_path
        )));
    }
    let text = decode_win1252(&header_block.payload[..text_end]);
    let params = ParameterCollection::from_string(&text);
    let header = params
        .get("HEADER")
        .map(|v| v.as_str().to_string())
        .ok_or_else(|| {
            AltiumError::Parse(format!(
                "schlib stream '{}' header block missing HEADER",
                stream_path
            ))
        })?;
    if !header.eq_ignore_ascii_case(expected_header) {
        return Err(AltiumError::Parse(format!(
            "schlib stream '{}' has unexpected HEADER='{}' (expected '{}')",
            stream_path, header, expected_header
        )));
    }

    let weight = if let Some(v) = params.get("Weight") {
        let parsed = v.as_int().map_err(|_| {
            AltiumError::Parse(format!(
                "schlib stream '{}' has invalid Weight='{}'",
                stream_path,
                v.as_str()
            ))
        })?;
        if parsed < 0 {
            return Err(AltiumError::Parse(format!(
                "schlib stream '{}' has negative Weight={}",
                stream_path, parsed
            )));
        }
        Some(parsed as usize)
    } else {
        None
    };

    if let Some(w) = weight {
        let expected = blocks.len().saturating_sub(1);
        if w != expected {
            return Err(AltiumError::Parse(format!(
                "schlib stream '{}' Weight={} does not match entry count {}",
                stream_path, w, expected
            )));
        }
    }

    let mut entries = Vec::with_capacity(blocks.len().saturating_sub(1));
    for block in &blocks[1..] {
        if block.flags != 0x01 {
            return Err(AltiumError::Parse(format!(
                "schlib stream '{}' block at offset {} has unsupported flags {}",
                stream_path, block.offset, block.flags
            )));
        }
        if block.payload.len() < 1 + 1 + 4 {
            return Err(AltiumError::Parse(format!(
                "schlib stream '{}' block at offset {} is too short",
                stream_path, block.offset
            )));
        }
        if block.payload[0] != CFB_COMPRESSED_TAG {
            return Err(AltiumError::Parse(format!(
                "schlib stream '{}' block at offset {} missing compressed tag 0xD0",
                stream_path, block.offset
            )));
        }

        let id_len = block.payload[1] as usize;
        let id_start = 2usize;
        let id_end = id_start + id_len;
        let hdr_end = id_end + 4;
        if hdr_end > block.payload.len() {
            return Err(AltiumError::Parse(format!(
                "schlib stream '{}' block at offset {} has invalid id length {}",
                stream_path, block.offset, id_len
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
                "schlib stream '{}' block at offset {} has invalid compressed length {}",
                stream_path, block.offset, inner_len
            )));
        }

        entries.push(SchLibEmbeddedObjectEntry {
            id,
            compressed_flags: inner_flags,
            compressed_data: block.payload[hdr_end..data_end].to_vec(),
        });
    }

    Ok(SchLibEmbeddedObjectStreamMeta {
        header,
        weight,
        params,
        entries,
    })
}

/// Serialize one typed SchLib component sidecar stream.
pub(crate) fn serialize_component_embedded_stream(
    meta: &SchLibEmbeddedObjectStreamMeta,
    stream_name: &str,
    expected_header: &str,
) -> Result<Vec<u8>> {
    let mut params = meta.params.clone();
    params.add("HEADER", expected_header);
    params.add("Weight", &meta.entries.len().to_string());
    let mut out = encode_single_param_block(&params);

    for entry in &meta.entries {
        let id_bytes = encode_win1252(&entry.id);
        if id_bytes.len() > 255 {
            return Err(AltiumError::Parse(format!(
                "schlib stream '{}' entry id too long: {} bytes",
                stream_name,
                id_bytes.len()
            )));
        }
        if entry.compressed_data.len() > SIZE_FLAG_MASK as usize {
            return Err(AltiumError::Parse(format!(
                "schlib stream '{}' entry '{}' compressed payload too large: {} bytes",
                stream_name,
                entry.id,
                entry.compressed_data.len()
            )));
        }

        let mut payload = Vec::new();
        payload.push(CFB_COMPRESSED_TAG);
        payload.push(id_bytes.len() as u8);
        payload.extend_from_slice(&id_bytes);
        let inner_header =
            ((entry.compressed_flags as u32) << 24) | (entry.compressed_data.len() as u32);
        payload.extend_from_slice(&inner_header.to_le_bytes());
        payload.extend_from_slice(&entry.compressed_data);

        let outer_header = (0x01u32 << 24) | (payload.len() as u32);
        out.extend_from_slice(&outer_header.to_le_bytes());
        out.extend_from_slice(&payload);
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_sidecar_roundtrip() {
        let mut params = ParameterCollection::new();
        params.add("HEADER", STREAM_PIN_FRAC);
        params.add("Weight", "1");

        let meta = SchLibEmbeddedObjectStreamMeta {
            header: STREAM_PIN_FRAC.to_string(),
            weight: Some(1),
            params,
            entries: vec![SchLibEmbeddedObjectEntry {
                id: "0".to_string(),
                compressed_flags: 0,
                compressed_data: vec![1, 2, 3, 4],
            }],
        };

        let data =
            serialize_component_embedded_stream(&meta, "U1/PinFrac", STREAM_PIN_FRAC).unwrap();
        let parsed = parse_component_embedded_stream(&data, "U1/PinFrac", STREAM_PIN_FRAC).unwrap();
        assert_eq!(parsed.header, STREAM_PIN_FRAC);
        assert_eq!(parsed.entries.len(), 1);
        assert_eq!(parsed.entries[0].id, "0");
        assert_eq!(parsed.entries[0].compressed_data, vec![1, 2, 3, 4]);
    }

    #[test]
    fn component_sidecar_rejects_wrong_header() {
        let mut params = ParameterCollection::new();
        params.add("HEADER", "Other");
        params.add("Weight", "0");
        let meta = SchLibEmbeddedObjectStreamMeta {
            header: "Other".to_string(),
            weight: Some(0),
            params,
            entries: Vec::new(),
        };

        let data = serialize_component_embedded_stream(&meta, "U1/PinFrac", "Other").unwrap();
        let err =
            parse_component_embedded_stream(&data, "U1/PinFrac", STREAM_PIN_FRAC).unwrap_err();
        assert!(format!("{err}").contains("unexpected HEADER"));
    }
}
