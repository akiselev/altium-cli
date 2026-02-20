//! Typed PcbDoc stream metadata and strict stream codecs.
//!
//! This module models PcbDoc stream families and provides strict parse/serialize
//! helpers for the known AD26 section shapes.

use std::collections::BTreeMap;

use crate::error::{AltiumError, Result};
use crate::ids::RecordId;
use crate::parameters::ParameterCollection;

use super::encoding::{decode_win1252, encode_win1252};

/// Typed metadata for one primitive section (`Tracks6`, `Pads6`, ...).
#[derive(Clone, Debug, Default)]
pub struct PcbDocPrimitiveSectionMeta {
    pub object_id: u8,
    pub header_count: u32,
    pub record_ids: Vec<RecordId>,
}

/// Typed metadata for a parameter-block section (`Board6`, `Nets6`, ...).
#[derive(Clone, Debug, Default)]
pub struct PcbDocParamSectionMeta {
    pub header_count: u32,
    pub entries: Vec<ParameterCollection>,
}

/// One prefixed parameter entry (`Rules6`, `Dimensions6`, ...).
#[derive(Clone, Debug)]
pub struct PcbDocPrefixedParamEntry {
    /// 2-byte prefix that appears before each parameter block.
    pub prefix: u16,
    pub params: ParameterCollection,
}

/// Typed metadata for prefixed parameter sections.
#[derive(Clone, Debug, Default)]
pub struct PcbDocPrefixedParamSectionMeta {
    pub header_count: u32,
    pub entries: Vec<PcbDocPrefixedParamEntry>,
}

/// Typed metadata for counted raw sections where payload stays binary.
#[derive(Clone, Debug, Default)]
pub struct PcbDocRawSectionMeta {
    pub header_count: u32,
    pub data: Vec<u8>,
}

/// Typed metadata for `Models/{Header,Data,<N>}`.
#[derive(Clone, Debug, Default)]
pub struct PcbDocModelsSectionMeta {
    pub header_count: u32,
    pub data: Vec<u8>,
    pub entries: BTreeMap<u32, Vec<u8>>,
}

/// Typed metadata for one `Section/Header` + `Section/Data` pair.
#[derive(Clone, Debug)]
pub enum PcbDocSectionMeta {
    Primitive(PcbDocPrimitiveSectionMeta),
    Param(PcbDocParamSectionMeta),
    PrefixedParam(PcbDocPrefixedParamSectionMeta),
    Raw(PcbDocRawSectionMeta),
    Models(PcbDocModelsSectionMeta),
}

impl PcbDocSectionMeta {
    /// Returns the header count this section will emit on save.
    pub fn header_count(&self) -> u32 {
        match self {
            Self::Primitive(m) => m.header_count,
            Self::Param(m) => m.header_count,
            Self::PrefixedParam(m) => m.header_count,
            Self::Raw(m) => m.header_count,
            Self::Models(m) => m.header_count,
        }
    }
}

/// Top-level typed PcbDoc non-record metadata.
#[derive(Clone, Debug, Default)]
pub struct PcbDocStreamsMeta {
    /// Root `FileHeader` stream bytes.
    pub file_header: Vec<u8>,
    /// Optional root `FileHeaderSix` stream bytes.
    pub file_header_six: Option<Vec<u8>>,
    /// Typed section metadata keyed by section storage name (`Board6`, ...).
    pub sections: BTreeMap<String, PcbDocSectionMeta>,
}

/// Internal known section kind classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PcbDocSectionKind {
    Primitive { object_id: u8 },
    Param,
    PrefixedParam,
    Raw,
    Models,
}

/// Classify a section storage by known AD26 semantics.
pub(crate) fn classify_section_kind(section_name: &str) -> Option<PcbDocSectionKind> {
    let lower = section_name.to_ascii_lowercase();
    match lower.as_str() {
        // Primitive sections (u8 type + payload blocks).
        "arcs6" => Some(PcbDocSectionKind::Primitive { object_id: 1 }),
        "pads6" => Some(PcbDocSectionKind::Primitive { object_id: 2 }),
        "vias6" => Some(PcbDocSectionKind::Primitive { object_id: 3 }),
        "tracks6" => Some(PcbDocSectionKind::Primitive { object_id: 4 }),
        "texts6" => Some(PcbDocSectionKind::Primitive { object_id: 5 }),
        "fills6" => Some(PcbDocSectionKind::Primitive { object_id: 6 }),
        "connections6" => Some(PcbDocSectionKind::Primitive { object_id: 7 }),
        "regions6" => Some(PcbDocSectionKind::Primitive { object_id: 11 }),
        "shapebasedregions6" => Some(PcbDocSectionKind::Primitive { object_id: 11 }),
        "splitplaneregions6" => Some(PcbDocSectionKind::Primitive { object_id: 11 }),
        "componentbodies6" => Some(PcbDocSectionKind::Primitive { object_id: 12 }),
        "shapebasedcomponentbodies6" => Some(PcbDocSectionKind::Primitive { object_id: 12 }),
        // Legacy primitive storages still present in some AD26 files.
        "boardregions" => Some(PcbDocSectionKind::Primitive { object_id: 11 }),
        "texts" => Some(PcbDocSectionKind::Primitive { object_id: 5 }),

        // Parameter sections (`u32 len + |KEY=VALUE|...`).
        "advanced placer options6" => Some(PcbDocSectionKind::Param),
        "advanced router options6" => Some(PcbDocSectionKind::Param),
        "board6" => Some(PcbDocSectionKind::Param),
        "classes6" => Some(PcbDocSectionKind::Param),
        "components6" => Some(PcbDocSectionKind::Param),
        "design rule checker options6" => Some(PcbDocSectionKind::Param),
        "differentialpairs6" => Some(PcbDocSectionKind::Param),
        "embeddedboards6" => Some(PcbDocSectionKind::Param),
        "embeddeds6" => Some(PcbDocSectionKind::Param),
        "extendedprimitiveinformation" => Some(PcbDocSectionKind::Param),
        "fromtos6" => Some(PcbDocSectionKind::Param),
        "nets6" => Some(PcbDocSectionKind::Param),
        "padvialibrary" => Some(PcbDocSectionKind::Param),
        "padvialibrarycache" => Some(PcbDocSectionKind::Param),
        "padvialibrarylinks" => Some(PcbDocSectionKind::Param),
        "pin swap options6" => Some(PcbDocSectionKind::Param),
        "pinpairssection" => Some(PcbDocSectionKind::Param),
        "polygons6" => Some(PcbDocSectionKind::Param),
        "primitiveparameters" => Some(PcbDocSectionKind::Param),
        "signalclasses" => Some(PcbDocSectionKind::Param),
        "smartunions" => Some(PcbDocSectionKind::Param),
        "unionrelations" => Some(PcbDocSectionKind::Param),
        "uniqueidprimitiveinformation" => Some(PcbDocSectionKind::Param),
        "waivedviolations" => Some(PcbDocSectionKind::Param),

        // Prefixed parameter sections (`u16 prefix + u32 len + params`).
        "rules6" => Some(PcbDocSectionKind::PrefixedParam),
        "newrules6" => Some(PcbDocSectionKind::PrefixedParam),
        "dimensions6" => Some(PcbDocSectionKind::PrefixedParam),
        "coordinates6" => Some(PcbDocSectionKind::PrefixedParam),

        // Counted raw sections.
        "embeddedfonts6" => Some(PcbDocSectionKind::Raw),
        "fileversioninfo" => Some(PcbDocSectionKind::Raw),
        "layerkindmapping" => Some(PcbDocSectionKind::Raw),
        "modelsnoembed" => Some(PcbDocSectionKind::Raw),
        "primitiveguids" => Some(PcbDocSectionKind::Raw),
        "textures" => Some(PcbDocSectionKind::Raw),
        "unionnames" => Some(PcbDocSectionKind::Raw),
        "widestrings6" => Some(PcbDocSectionKind::Raw),
        "constraintmanager" => Some(PcbDocSectionKind::Raw),

        // Models storage has numbered substreams in addition to Header/Data.
        "models" => Some(PcbDocSectionKind::Models),
        _ => None,
    }
}

/// Parse strict 4-byte header count stream.
pub(crate) fn parse_u32_header_stream(data: &[u8], stream_path: &str) -> Result<u32> {
    if data.len() != 4 {
        return Err(AltiumError::Parse(format!(
            "pcbdoc stream '{}' expected 4-byte header, got {} bytes",
            stream_path,
            data.len()
        )));
    }
    Ok(u32::from_le_bytes([data[0], data[1], data[2], data[3]]))
}

/// Serialize 4-byte header count stream.
pub(crate) fn serialize_u32_header_stream(count: u32) -> [u8; 4] {
    count.to_le_bytes()
}

fn decode_param_payload(
    payload: &[u8],
    stream_path: &str,
    index: usize,
) -> Result<ParameterCollection> {
    let text_end = payload
        .iter()
        .position(|b| *b == 0)
        .unwrap_or(payload.len());
    if payload[text_end..].iter().any(|b| *b != 0) {
        return Err(AltiumError::Parse(format!(
            "pcbdoc stream '{}' record {} has non-zero bytes after NUL terminator",
            stream_path, index
        )));
    }
    let text = decode_win1252(&payload[..text_end]);
    Ok(ParameterCollection::from_string(&text))
}

/// Parse `u32 len + payload` parameter blocks.
pub(crate) fn parse_param_section_data(
    data: &[u8],
    stream_path: &str,
) -> Result<Vec<ParameterCollection>> {
    let mut off = 0usize;
    let mut out = Vec::new();
    let mut index = 0usize;
    while off < data.len() {
        if off + 4 > data.len() {
            return Err(AltiumError::Parse(format!(
                "pcbdoc stream '{}' truncated before record {} length",
                stream_path, index
            )));
        }
        let len =
            u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]]) as usize;
        off += 4;
        if len == 0 {
            return Err(AltiumError::Parse(format!(
                "pcbdoc stream '{}' has zero-length record {}",
                stream_path, index
            )));
        }
        if off + len > data.len() {
            return Err(AltiumError::Parse(format!(
                "pcbdoc stream '{}' record {} overflows stream (len={})",
                stream_path, index, len
            )));
        }
        let payload = &data[off..off + len];
        off += len;
        out.push(decode_param_payload(payload, stream_path, index)?);
        index += 1;
    }
    Ok(out)
}

/// Serialize `u32 len + payload` parameter blocks.
pub(crate) fn serialize_param_section_data(entries: &[ParameterCollection]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    for (i, params) in entries.iter().enumerate() {
        let payload = encode_win1252(&params.to_param_string());
        let len = u32::try_from(payload.len()).map_err(|_| {
            AltiumError::Parse(format!(
                "pcbdoc parameter record {} too large: {} bytes",
                i,
                payload.len()
            ))
        })?;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&payload);
    }
    Ok(out)
}

/// Parse `u16 prefix + u32 len + payload` parameter blocks.
pub(crate) fn parse_prefixed_param_section_data(
    data: &[u8],
    stream_path: &str,
) -> Result<Vec<PcbDocPrefixedParamEntry>> {
    let mut off = 0usize;
    let mut out = Vec::new();
    let mut index = 0usize;
    while off < data.len() {
        if off + 2 + 4 > data.len() {
            return Err(AltiumError::Parse(format!(
                "pcbdoc stream '{}' truncated before prefixed record {} header",
                stream_path, index
            )));
        }
        let prefix = u16::from_le_bytes([data[off], data[off + 1]]);
        off += 2;
        let len =
            u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]]) as usize;
        off += 4;
        if len == 0 {
            return Err(AltiumError::Parse(format!(
                "pcbdoc stream '{}' has zero-length prefixed record {}",
                stream_path, index
            )));
        }
        if off + len > data.len() {
            return Err(AltiumError::Parse(format!(
                "pcbdoc stream '{}' prefixed record {} overflows stream (len={})",
                stream_path, index, len
            )));
        }
        let payload = &data[off..off + len];
        off += len;
        out.push(PcbDocPrefixedParamEntry {
            prefix,
            params: decode_param_payload(payload, stream_path, index)?,
        });
        index += 1;
    }
    Ok(out)
}

/// Serialize `u16 prefix + u32 len + payload` parameter blocks.
pub(crate) fn serialize_prefixed_param_section_data(
    entries: &[PcbDocPrefixedParamEntry],
) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    for (i, entry) in entries.iter().enumerate() {
        let payload = encode_win1252(&entry.params.to_param_string());
        let len = u32::try_from(payload.len()).map_err(|_| {
            AltiumError::Parse(format!(
                "pcbdoc prefixed parameter record {} too large: {} bytes",
                i,
                payload.len()
            ))
        })?;
        out.extend_from_slice(&entry.prefix.to_le_bytes());
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&payload);
    }
    Ok(out)
}
