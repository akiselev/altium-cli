//! PcbDoc document I/O using the v2 `DocumentStore` architecture.
//!
//! A modern AD26 PcbDoc is a CFB document with:
//! - root streams (`/FileHeader`, optional `/FileHeaderSix`)
//! - section storages with `Header` + `Data` streams (e.g. `Tracks6/*`)
//! - optional section-specific extra streams (e.g. `Models/<N>`).
//!
//! This module parses known AD26 sections strictly and errors on unknown or
//! malformed stream layouts.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::{Cursor, Read, Seek, Write};
use std::rc::Rc;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::backing_store::{RecordNode, RecordOrigin};
use crate::error::{AltiumError, Result};
use crate::ids::RecordId;
use crate::records::{
    parse_arc, parse_component_body, parse_connection, parse_fill, parse_pad, parse_region,
    parse_text, parse_track, parse_via,
};
use crate::store::{DocRef, DocumentMeta, DocumentStore};
use crate::traits::{FromOrigin, HandleFamily, RecordType};

use super::pcbdoc_streams::{
    PcbDocModelsSectionMeta, PcbDocParamSectionMeta, PcbDocPrefixedParamSectionMeta,
    PcbDocPrimitiveSectionMeta, PcbDocRawSectionMeta, PcbDocSectionKind, PcbDocSectionMeta,
    PcbDocStreamsMeta, classify_section_kind, parse_param_section_data,
    parse_prefixed_param_section_data, parse_u32_header_stream, serialize_param_section_data,
    serialize_prefixed_param_section_data, serialize_u32_header_stream,
};

const STREAM_FILE_HEADER: &str = "FileHeader";
const STREAM_FILE_HEADER_SIX: &str = "FileHeaderSix";

#[derive(Default)]
struct PcbDocSectionPaths {
    header: Option<String>,
    data: Option<String>,
    numbered_entries: BTreeMap<u32, String>,
}

/// A parsed PcbDoc document using the v2 store architecture.
pub struct PcbDoc {
    store: DocRef,
}

impl PcbDoc {
    /// Create a new empty PcbDoc document.
    pub fn new_empty() -> Self {
        let mut header_text_utf16le = Vec::new();
        for ch in "PCB 5.0 Binary File".encode_utf16() {
            header_text_utf16le.extend_from_slice(&ch.to_le_bytes());
        }
        let mut file_header = Vec::new();
        file_header.extend_from_slice(&19u32.to_le_bytes());
        file_header.extend_from_slice(&header_text_utf16le);

        let mut store = DocumentStore::new(DocumentMeta::PcbDoc {
            streams_meta: PcbDocStreamsMeta {
                file_header,
                file_header_six: None,
                sections: BTreeMap::new(),
            },
        });
        store.set_semantic_context("dtid:pcbdoc", "");
        Self {
            store: Rc::new(RefCell::new(store)),
        }
    }

    /// Returns typed stream metadata.
    pub fn streams_meta(&self) -> PcbDocStreamsMeta {
        let store = self.store.borrow();
        match store.meta() {
            DocumentMeta::PcbDoc { streams_meta } => streams_meta.clone(),
            _ => PcbDocStreamsMeta::default(),
        }
    }

    /// Replace typed stream metadata.
    pub fn set_streams_meta(&self, streams_meta: PcbDocStreamsMeta) -> Result<()> {
        let mut store = self.store.borrow_mut();
        match store.meta_mut() {
            DocumentMeta::PcbDoc {
                streams_meta: current,
            } => {
                *current = streams_meta;
                store.mark_semantic_ids_dirty();
                Ok(())
            }
            other => Err(AltiumError::TypeMismatch(format!(
                "expected PcbDoc, got {}",
                other.variant_name()
            ))),
        }
    }

    /// Construct a typed handle for a record in this document's store.
    pub fn handle_for<H: HandleFamily>(&self, rid: RecordId) -> Result<H::Handle> {
        H::try_make_handle(self.store.clone(), rid)
    }

    /// Returns primitive records for one section (`Tracks6`, `Pads6`, ...).
    pub fn primitive_records(&self, section_name: &str) -> Vec<(u8, RecordId)> {
        let store = self.store.borrow();
        let Some(meta) = (match store.meta() {
            DocumentMeta::PcbDoc { streams_meta } => streams_meta.sections.get(section_name),
            _ => None,
        }) else {
            return Vec::new();
        };

        let PcbDocSectionMeta::Primitive(p) = meta else {
            return Vec::new();
        };

        p.record_ids
            .iter()
            .copied()
            .map(|rid| (store.record(rid).key, rid))
            .collect()
    }

    /// Add a typed primitive record to an existing primitive section.
    ///
    /// Returns the inserted record id.
    pub fn add_primitive_record<R>(&self, section_name: &str, record: R) -> Result<RecordId>
    where
        R: FromOrigin + RecordType,
    {
        if !R::IS_BINARY {
            return Err(AltiumError::Parse(format!(
                "pcbdoc section '{}' requires binary primitive records (got param record_id={})",
                section_name,
                R::RECORD_ID
            )));
        }

        {
            let store = self.store.borrow();
            let Some(section_meta) = (match store.meta() {
                DocumentMeta::PcbDoc { streams_meta } => streams_meta.sections.get(section_name),
                _ => None,
            }) else {
                return Err(AltiumError::Parse(format!(
                    "pcbdoc primitive section '{}' does not exist",
                    section_name
                )));
            };

            let PcbDocSectionMeta::Primitive(primitive_meta) = section_meta else {
                return Err(AltiumError::Parse(format!(
                    "pcbdoc section '{}' is not a primitive section",
                    section_name
                )));
            };

            if primitive_meta.object_id != R::RECORD_ID {
                return Err(AltiumError::Parse(format!(
                    "pcbdoc section '{}' expects object_id={} but got record_id={}",
                    section_name,
                    primitive_meta.object_id,
                    R::RECORD_ID
                )));
            }
        }

        let mut node = RecordNode::new(R::RECORD_ID, record.into_origin());
        node.stream_name = Some(section_name.to_string());
        node.mark_dirty();

        let mut store = self.store.borrow_mut();
        let rid = store.insert_record(node);
        store.orphan_records.push(rid);
        store.orphan_original_indices.push(usize::MAX);

        if let DocumentMeta::PcbDoc { streams_meta } = store.meta_mut() {
            let Some(PcbDocSectionMeta::Primitive(primitive_meta)) =
                streams_meta.sections.get_mut(section_name)
            else {
                return Err(AltiumError::Parse(format!(
                    "pcbdoc primitive section '{}' disappeared during insert",
                    section_name
                )));
            };
            primitive_meta.record_ids.push(rid);
        } else {
            return Err(AltiumError::Parse(
                "pcbdoc add_primitive_record called on non-PcbDoc document".to_string(),
            ));
        }

        store.mark_semantic_ids_dirty();
        Ok(rid)
    }

    /// Query a single primitive record of type `T` across all primitive sections.
    pub fn query_primitive<T: HandleFamily>(&self, q: &str) -> Result<T::Handle> {
        use crate::query::eval::evaluate;
        let parsed = crate::query::parse(q)?;

        let store = self.store.borrow();
        let mut matches = Vec::new();
        for &rid in store.orphan_ids() {
            let node = store.record(rid);
            if node.key == T::record_id() && node.origin.is_binary() == T::is_binary() {
                let all = std::slice::from_ref(node);
                if !evaluate(&parsed, all).is_empty() {
                    matches.push(rid);
                }
            }
        }
        drop(store);

        match matches.len() {
            0 => Err(AltiumError::NoMatch(q.to_string())),
            1 => T::try_make_handle(self.store.clone(), matches[0]),
            n => Err(AltiumError::AmbiguousMatch(n, q.to_string())),
        }
    }

    /// Query all primitive records of type `T` across all primitive sections.
    pub fn query_all_primitives<T: HandleFamily>(&self, q: &str) -> Result<Vec<T::Handle>> {
        use crate::query::eval::evaluate;
        let parsed = crate::query::parse(q)?;

        let store = self.store.borrow();
        let mut matches = Vec::new();
        for &rid in store.orphan_ids() {
            let node = store.record(rid);
            if node.key == T::record_id() && node.origin.is_binary() == T::is_binary() {
                let all = std::slice::from_ref(node);
                if !evaluate(&parsed, all).is_empty() {
                    matches.push(rid);
                }
            }
        }
        drop(store);

        matches
            .into_iter()
            .map(|rid| T::try_make_handle(self.store.clone(), rid))
            .collect()
    }

    /// Open a PcbDoc from a reader.
    pub fn open<R: Read + Seek>(mut reader: R) -> Result<Self> {
        let mut raw_bytes = Vec::new();
        reader
            .read_to_end(&mut raw_bytes)
            .map_err(AltiumError::Io)?;
        let doc_key = crate::semantic_ids::blake3_content_hash(&raw_bytes);

        let mut cfb = cfb::CompoundFile::open(Cursor::new(raw_bytes))
            .map_err(|e| AltiumError::Cfb(format!("Failed to open CFB: {}", e)))?;

        let (file_header_path, file_header_six_path, section_paths) =
            collect_pcbdoc_stream_paths(&cfb)?;

        let file_header = read_stream_bytes(&mut cfb, &file_header_path)?;
        let file_header_six = match file_header_six_path {
            Some(path) => Some(read_stream_bytes(&mut cfb, &path)?),
            None => None,
        };

        let mut streams_meta = PcbDocStreamsMeta {
            file_header,
            file_header_six,
            sections: BTreeMap::new(),
        };

        let mut store = DocumentStore::new(DocumentMeta::PcbDoc {
            streams_meta: PcbDocStreamsMeta::default(),
        });
        let mut orphan_original_index = 0usize;

        for (section_name, paths) in section_paths {
            let Some(header_path) = paths.header else {
                return Err(AltiumError::Parse(format!(
                    "pcbdoc section '{}' missing Header stream",
                    section_name
                )));
            };
            let Some(data_path) = paths.data else {
                return Err(AltiumError::Parse(format!(
                    "pcbdoc section '{}' missing Data stream",
                    section_name
                )));
            };

            let kind = classify_section_kind(&section_name).ok_or_else(|| {
                AltiumError::Parse(format!(
                    "pcbdoc contains unimplemented section '{}'",
                    section_name
                ))
            })?;

            let header_data = read_stream_bytes(&mut cfb, &header_path)?;
            let data = read_stream_bytes(&mut cfb, &data_path)?;
            let header_count = parse_u32_header_stream(&header_data, &header_path)?;

            let section_meta = match kind {
                PcbDocSectionKind::Primitive { object_id } => {
                    let records = parse_primitive_section_data(&data, &section_name, object_id)?;
                    let mut record_ids = Vec::with_capacity(records.len());
                    for mut node in records {
                        node.stream_name = Some(section_name.clone());
                        let rid = store.insert_record(node);
                        store.orphan_records.push(rid);
                        store.orphan_original_indices.push(orphan_original_index);
                        orphan_original_index += 1;
                        record_ids.push(rid);
                    }
                    PcbDocSectionMeta::Primitive(PcbDocPrimitiveSectionMeta {
                        object_id,
                        header_count,
                        record_ids,
                    })
                }
                PcbDocSectionKind::Param => {
                    let entries = parse_param_section_data(&data, &data_path)?;
                    PcbDocSectionMeta::Param(PcbDocParamSectionMeta {
                        header_count,
                        entries,
                    })
                }
                PcbDocSectionKind::PrefixedParam => {
                    let entries = parse_prefixed_param_section_data(&data, &data_path)?;
                    PcbDocSectionMeta::PrefixedParam(PcbDocPrefixedParamSectionMeta {
                        header_count,
                        entries,
                    })
                }
                PcbDocSectionKind::Raw => {
                    PcbDocSectionMeta::Raw(PcbDocRawSectionMeta { header_count, data })
                }
                PcbDocSectionKind::Models => {
                    let mut entries = BTreeMap::new();
                    for (idx, model_path) in paths.numbered_entries {
                        entries.insert(idx, read_stream_bytes(&mut cfb, &model_path)?);
                    }
                    PcbDocSectionMeta::Models(PcbDocModelsSectionMeta {
                        header_count,
                        data,
                        entries,
                    })
                }
            };

            streams_meta.sections.insert(section_name, section_meta);
        }

        store.set_semantic_context("dtid:pcbdoc", &doc_key);
        crate::semantic_ids::compute_all_ids(&mut store, "dtid:pcbdoc", &doc_key)?;

        if let DocumentMeta::PcbDoc {
            streams_meta: current,
        } = store.meta_mut()
        {
            *current = streams_meta;
        }

        Ok(Self {
            store: Rc::new(RefCell::new(store)),
        })
    }

    /// Open from a file path.
    pub fn open_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let file = std::fs::File::open(path).map_err(AltiumError::Io)?;
        Self::open(file)
    }

    /// Save to a writer.
    pub fn save<W: Read + Write + Seek>(&self, writer: W) -> Result<()> {
        {
            let mut store = self.store.borrow_mut();
            store.ensure_semantic_ids()?;
        }

        let mut cfb = cfb::CompoundFile::create_with_version(cfb::Version::V3, writer)
            .map_err(|e| AltiumError::Cfb(format!("Failed to create CFB: {}", e)))?;

        let (streams_meta, store_ref) = {
            let store = self.store.borrow();
            let streams_meta = match store.meta() {
                DocumentMeta::PcbDoc { streams_meta } => streams_meta.clone(),
                _ => {
                    return Err(AltiumError::Parse(
                        "pcbdoc save called on non-PcbDoc document".to_string(),
                    ));
                }
            };
            (streams_meta, self.store.clone())
        };

        write_stream_bytes(&mut cfb, STREAM_FILE_HEADER, &streams_meta.file_header)?;
        if let Some(file_header_six) = streams_meta.file_header_six.as_ref() {
            write_stream_bytes(&mut cfb, STREAM_FILE_HEADER_SIX, file_header_six)?;
        }

        for (section_name, section_meta) in &streams_meta.sections {
            cfb.create_storage(format!("/{}", section_name))
                .map_err(|e| {
                    AltiumError::Cfb(format!(
                        "Failed to create pcbdoc storage '/{}': {}",
                        section_name, e
                    ))
                })?;

            let header_bytes = serialize_u32_header_stream(section_meta.header_count());
            write_stream_bytes(&mut cfb, &format!("{}/Header", section_name), &header_bytes)?;

            let data_bytes = match section_meta {
                PcbDocSectionMeta::Primitive(meta) => {
                    build_primitive_section_data(&store_ref, section_name, meta)?
                }
                PcbDocSectionMeta::Param(meta) => serialize_param_section_data(&meta.entries)?,
                PcbDocSectionMeta::PrefixedParam(meta) => {
                    serialize_prefixed_param_section_data(&meta.entries)?
                }
                PcbDocSectionMeta::Raw(meta) => meta.data.clone(),
                PcbDocSectionMeta::Models(meta) => {
                    for (idx, bytes) in &meta.entries {
                        write_stream_bytes(&mut cfb, &format!("{}/{}", section_name, idx), bytes)?;
                    }
                    meta.data.clone()
                }
            };
            write_stream_bytes(&mut cfb, &format!("{}/Data", section_name), &data_bytes)?;
        }

        cfb.flush()
            .map_err(|e| AltiumError::Cfb(format!("CFB flush: {}", e)))?;
        Ok(())
    }

    /// Save to a file path.
    pub fn save_file(&self, path: impl AsRef<std::path::Path>) -> Result<()> {
        let file = std::fs::File::create(path).map_err(AltiumError::Io)?;
        self.save(file)
    }
}

fn collect_pcbdoc_stream_paths<R: Read + Seek>(
    cfb: &cfb::CompoundFile<R>,
) -> Result<(String, Option<String>, BTreeMap<String, PcbDocSectionPaths>)> {
    let mut file_header: Option<String> = None;
    let mut file_header_six: Option<String> = None;
    let mut sections: BTreeMap<String, PcbDocSectionPaths> = BTreeMap::new();

    for entry in cfb.walk().filter(|e| e.is_stream()) {
        let path = entry
            .path()
            .to_str()
            .ok_or_else(|| AltiumError::Parse("pcbdoc contains non-UTF8 stream path".to_string()))?
            .to_string();

        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        match parts.as_slice() {
            [root] => {
                if root.eq_ignore_ascii_case(STREAM_FILE_HEADER) {
                    if file_header.is_some() {
                        return Err(AltiumError::Parse(format!(
                            "pcbdoc contains duplicate '{}' stream",
                            STREAM_FILE_HEADER
                        )));
                    }
                    file_header = Some(path);
                } else if root.eq_ignore_ascii_case(STREAM_FILE_HEADER_SIX) {
                    if file_header_six.is_some() {
                        return Err(AltiumError::Parse(format!(
                            "pcbdoc contains duplicate '{}' stream",
                            STREAM_FILE_HEADER_SIX
                        )));
                    }
                    file_header_six = Some(path);
                } else {
                    return Err(AltiumError::Parse(format!(
                        "pcbdoc contains unimplemented stream '{}'",
                        path
                    )));
                }
            }
            [section, leaf] => {
                let kind = classify_section_kind(section).ok_or_else(|| {
                    AltiumError::Parse(format!("pcbdoc contains unimplemented stream '{}'", path))
                })?;

                let section_entry = sections.entry((*section).to_string()).or_default();
                if leaf.eq_ignore_ascii_case("Header") {
                    if section_entry.header.is_some() {
                        return Err(AltiumError::Parse(format!(
                            "pcbdoc contains duplicate section Header stream '{}'",
                            path
                        )));
                    }
                    section_entry.header = Some(path);
                } else if leaf.eq_ignore_ascii_case("Data") {
                    if section_entry.data.is_some() {
                        return Err(AltiumError::Parse(format!(
                            "pcbdoc contains duplicate section Data stream '{}'",
                            path
                        )));
                    }
                    section_entry.data = Some(path);
                } else if matches!(kind, PcbDocSectionKind::Models) {
                    let idx = leaf.parse::<u32>().map_err(|_| {
                        AltiumError::Parse(format!(
                            "pcbdoc Models stream '{}' is not a numeric entry",
                            path
                        ))
                    })?;
                    if section_entry.numbered_entries.insert(idx, path).is_some() {
                        return Err(AltiumError::Parse(format!(
                            "pcbdoc contains duplicate Models entry {}",
                            idx
                        )));
                    }
                } else {
                    return Err(AltiumError::Parse(format!(
                        "pcbdoc contains unimplemented stream '{}'",
                        path
                    )));
                }
            }
            _ => {
                return Err(AltiumError::Parse(format!(
                    "pcbdoc contains nested stream '{}'",
                    path
                )));
            }
        }
    }

    let file_header = file_header.ok_or_else(|| {
        AltiumError::Parse(format!(
            "pcbdoc missing required '{}' stream",
            STREAM_FILE_HEADER
        ))
    })?;

    Ok((file_header, file_header_six, sections))
}

fn read_stream_bytes<R: Read + Seek>(
    cfb: &mut cfb::CompoundFile<R>,
    path: &str,
) -> Result<Vec<u8>> {
    let mut stream = cfb
        .open_stream(path)
        .map_err(|e| AltiumError::Cfb(format!("Failed to open {}: {}", path, e)))?;
    let mut data = Vec::new();
    stream.read_to_end(&mut data).map_err(AltiumError::Io)?;
    Ok(data)
}

fn write_stream_bytes<R: Read + Write + Seek>(
    cfb: &mut cfb::CompoundFile<R>,
    stream_path: &str,
    bytes: &[u8],
) -> Result<()> {
    let mut stream = cfb
        .create_stream(format!("/{}", stream_path))
        .map_err(|e| AltiumError::Cfb(format!("Failed to create {}: {}", stream_path, e)))?;
    stream.write_all(bytes).map_err(AltiumError::Io)?;
    Ok(())
}

fn subrecord_count(type_byte: u8) -> usize {
    match type_byte {
        2 => 6, // Pad
        5 => 2, // Text
        _ => 1,
    }
}

fn parse_single_subrecord_origin(type_byte: u8, block_data: Vec<u8>) -> Result<RecordOrigin> {
    match type_byte {
        1 => parse_arc(&block_data),
        3 => parse_via(&block_data),
        4 => parse_track(&block_data),
        6 => parse_fill(&block_data),
        7 => parse_connection(&block_data),
        11 => parse_region(&block_data),
        12 => parse_component_body(&block_data),
        _ => Err(AltiumError::Parse(format!(
            "pcbdoc unimplemented single-subrecord object_id={}",
            type_byte
        ))),
    }
}

fn parse_multi_subrecord_origin(type_byte: u8, block_data: Vec<u8>) -> Result<RecordOrigin> {
    match type_byte {
        2 => parse_pad(&block_data),
        5 => parse_text(&block_data),
        _ => Err(AltiumError::Parse(format!(
            "pcbdoc unimplemented multi-subrecord object_id={}",
            type_byte
        ))),
    }
}

fn parse_primitive_section_data(
    data: &[u8],
    section_name: &str,
    expected_object_id: u8,
) -> Result<Vec<RecordNode>> {
    let mut cursor = Cursor::new(data);
    let mut records = Vec::new();
    let mut i = 0usize;
    while (cursor.position() as usize) < data.len() {
        let type_byte = cursor.read_u8().map_err(|_| {
            AltiumError::Parse(format!(
                "pcbdoc section '{}' truncated before object {} type byte",
                section_name, i
            ))
        })?;
        if type_byte != expected_object_id {
            return Err(AltiumError::Parse(format!(
                "pcbdoc section '{}' object {} has object_id={} (expected {})",
                section_name, i, type_byte, expected_object_id
            )));
        }

        let n = subrecord_count(type_byte);
        let origin = if n == 1 {
            let block_len = cursor.read_u32::<LittleEndian>().map_err(|_| {
                AltiumError::Parse(format!(
                    "pcbdoc section '{}' truncated before object {} length",
                    section_name, i
                ))
            })? as usize;
            if cursor.position() as usize + block_len > data.len() {
                return Err(AltiumError::Parse(format!(
                    "pcbdoc section '{}' object {} payload overflows stream (len={})",
                    section_name, i, block_len
                )));
            }
            let mut block_data = vec![0u8; block_len];
            cursor.read_exact(&mut block_data).map_err(|_| {
                AltiumError::Parse(format!(
                    "pcbdoc section '{}' truncated reading object {} payload",
                    section_name, i
                ))
            })?;
            parse_single_subrecord_origin(type_byte, block_data)?
        } else {
            let start = cursor.position() as usize;
            for sub in 0..n {
                let len = cursor.read_u32::<LittleEndian>().map_err(|_| {
                    AltiumError::Parse(format!(
                        "pcbdoc section '{}' truncated before object {} subrecord {} length",
                        section_name, i, sub
                    ))
                })? as usize;
                if cursor.position() as usize + len > data.len() {
                    return Err(AltiumError::Parse(format!(
                        "pcbdoc section '{}' object {} subrecord {} overflows stream (len={})",
                        section_name, i, sub, len
                    )));
                }
                cursor.set_position(cursor.position() + len as u64);
            }
            let end = cursor.position() as usize;
            let block_data = data[start..end].to_vec();
            parse_multi_subrecord_origin(type_byte, block_data)?
        };

        records.push(RecordNode::new(type_byte, origin));
        i += 1;
    }

    Ok(records)
}

fn build_primitive_section_data(
    store_ref: &DocRef,
    section_name: &str,
    meta: &PcbDocPrimitiveSectionMeta,
) -> Result<Vec<u8>> {
    let store = store_ref.borrow();
    let mut out = Vec::new();
    for (i, rid) in meta.record_ids.iter().enumerate() {
        let node = store.record(*rid);
        if node.key != meta.object_id {
            return Err(AltiumError::Parse(format!(
                "pcbdoc section '{}' record {} has object_id={} (expected {})",
                section_name, i, node.key, meta.object_id
            )));
        }

        let bytes: &[u8] = if node.is_dirty() {
            match &node.origin {
                RecordOrigin::Binary(b) => &b.raw_block,
                RecordOrigin::Param(_) => {
                    return Err(AltiumError::Parse(format!(
                        "pcbdoc section '{}' record {} is param-origin (expected binary)",
                        section_name, i
                    )));
                }
            }
        } else {
            node.snapshot_bytes()
        };

        out.push(node.key);
        if subrecord_count(node.key) == 1 {
            out.write_u32::<LittleEndian>(bytes.len() as u32)
                .map_err(AltiumError::Io)?;
            out.extend_from_slice(bytes);
        } else {
            out.extend_from_slice(bytes);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handles::PcbTrack;
    use crate::templates;

    fn build_test_cfb(streams: &[(&str, &[u8])]) -> Vec<u8> {
        let cursor = Cursor::new(Vec::<u8>::new());
        let mut cfb =
            cfb::CompoundFile::create_with_version(cfb::Version::V3, cursor).expect("create cfb");
        for (path, data) in streams {
            if let Some((storage, _)) = path.rsplit_once('/') {
                if !storage.is_empty() {
                    let _ = cfb.create_storage(format!("/{}", storage));
                }
            }
            let mut stream = cfb
                .create_stream(format!("/{}", path))
                .expect("create stream");
            stream.write_all(data).expect("write stream");
        }
        cfb.flush().expect("flush cfb");
        cfb.into_inner().into_inner()
    }

    #[test]
    fn open_rejects_unknown_root_streams() {
        let file_header = [19u32.to_le_bytes().as_slice(), b"P\0C\0B\0"].concat();
        let bytes = build_test_cfb(&[(STREAM_FILE_HEADER, &file_header), ("Unknown", b"x")]);
        let err = PcbDoc::open(Cursor::new(bytes))
            .err()
            .expect("expected open error");
        assert!(format!("{err}").contains("unimplemented stream"));
    }

    #[test]
    fn open_save_roundtrip_single_track_section() {
        let track_origin = templates::pcb_track_default();
        let payload = match track_origin {
            RecordOrigin::Binary(crate::backing_store::BinaryOrigin { raw_block, .. }) => raw_block,
            _ => panic!("unexpected non-binary track template"),
        };

        let mut tracks_data = Vec::new();
        tracks_data.push(4u8);
        tracks_data.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        tracks_data.extend_from_slice(&payload);

        let file_header = [19u32.to_le_bytes().as_slice(), b"P\0C\0B\0"].concat();
        let bytes = build_test_cfb(&[
            (STREAM_FILE_HEADER, &file_header),
            ("Tracks6/Header", &1u32.to_le_bytes()),
            ("Tracks6/Data", &tracks_data),
        ]);

        let doc = PcbDoc::open(Cursor::new(bytes)).expect("open pcbdoc");
        let section_records = doc.primitive_records("Tracks6");
        assert_eq!(section_records.len(), 1);
        assert_eq!(section_records[0].0, 4);

        let mut out = Cursor::new(Vec::new());
        doc.save(&mut out).expect("save pcbdoc");
        let reopened = PcbDoc::open(Cursor::new(out.into_inner())).expect("reopen pcbdoc");
        assert_eq!(reopened.primitive_records("Tracks6").len(), 1);
    }

    #[test]
    fn add_primitive_record_and_query_roundtrip() {
        let doc = PcbDoc::new_empty();
        let mut meta = doc.streams_meta();
        meta.sections.insert(
            "Tracks6".to_string(),
            PcbDocSectionMeta::Primitive(PcbDocPrimitiveSectionMeta {
                object_id: 4,
                header_count: 1,
                record_ids: Vec::new(),
            }),
        );
        doc.set_streams_meta(meta).expect("set_streams_meta");

        let track = crate::records::PcbTrackRecord::from_origin(templates::pcb_track_default());
        doc.add_primitive_record("Tracks6", track)
            .expect("add track primitive");

        let tracks = doc.primitive_records("Tracks6");
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].0, 4);

        let query_all = doc
            .query_all_primitives::<PcbTrack>("#4")
            .expect("query all tracks");
        assert_eq!(query_all.len(), 1);

        let query_one = doc
            .query_primitive::<PcbTrack>("#4")
            .expect("query single track");
        assert_eq!(query_one.read().track_kind(), 0);

        let mut out = Cursor::new(Vec::new());
        doc.save(&mut out).expect("save pcbdoc");
        let reopened = PcbDoc::open(Cursor::new(out.into_inner())).expect("reopen pcbdoc");
        assert_eq!(reopened.primitive_records("Tracks6").len(), 1);
    }

    #[test]
    fn add_primitive_record_rejects_mismatched_section_type() {
        let doc = PcbDoc::new_empty();
        let mut meta = doc.streams_meta();
        meta.sections.insert(
            "Tracks6".to_string(),
            PcbDocSectionMeta::Primitive(PcbDocPrimitiveSectionMeta {
                object_id: 4,
                header_count: 0,
                record_ids: Vec::new(),
            }),
        );
        doc.set_streams_meta(meta).expect("set_streams_meta");

        let arc = crate::records::PcbArcRecord::from_origin(templates::pcb_arc_default());
        let err = doc
            .add_primitive_record("Tracks6", arc)
            .expect_err("expected mismatched object_id error");
        assert!(
            err.to_string().contains("expects object_id"),
            "unexpected error: {err}"
        );
    }
}
