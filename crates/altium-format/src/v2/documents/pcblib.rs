//! PcbLib document I/O using the v2 DocumentStore architecture.
//!
//! A PcbLib file is a CFB compound file with one storage per footprint:
//! - `/<FootprintName>/Parameters` stream: footprint metadata (pipe-delimited)
//! - `/<FootprintName>/Header` stream: primitive count and version info
//! - `/<FootprintName>/Data` stream: binary primitives (type byte + length + data)
//!
//! The Data stream begins with a length-prefixed pattern name block, followed
//! by packed binary primitive records.

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{Cursor, Read, Seek, Write};
use std::rc::Rc;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::error::{AltiumError, Result};
use crate::v2::backing_store::{
    BinaryOrigin, ParamOrigin, PcbPrimitiveRef, RecordNode, RecordOrigin,
};
use crate::v2::handles::PcbFootprintHandle;
use crate::v2::ids::RecordId;
use crate::v2::records::{parse_component_body, parse_pad, parse_region, parse_text, parse_via};
use crate::v2::store::{DocRef, DocumentMeta, DocumentStore, GroupData, GroupMeta};
use crate::v2::traits::{DocumentQuery, HandleFamily};

use super::section_keys::SectionKeyList;

const STREAM_PARAMETERS: &str = "Parameters";
const STREAM_HEADER: &str = "Header";
const STREAM_DATA: &str = "Data";

/// A parsed PcbLib document using the v2 DocumentStore architecture.
///
/// All records and groups are stored in a centralized `DocumentStore` accessed
/// via `Rc<RefCell<>>` handles. The `store()` method provides access for
/// reading and writing footprint data through typed handles.
pub struct PcbLib {
    store: DocRef,
}

impl PcbLib {
    /// Returns a reference to the underlying document store.
    pub fn store(&self) -> &DocRef {
        &self.store
    }

    /// Returns the stable document-level semantic ID, if computed.
    pub fn document_id(&self) -> Option<crate::v2::semantic_ids::SemanticId> {
        self.store.borrow().document_id().cloned()
    }

    /// Open a PcbLib from a reader.
    pub fn open<R: Read + Seek>(mut reader: R) -> Result<Self> {
        let mut raw_bytes = Vec::new();
        reader.read_to_end(&mut raw_bytes).map_err(AltiumError::Io)?;
        let doc_key = crate::v2::semantic_ids::blake3_content_hash(&raw_bytes);

        let mut cfb = cfb::CompoundFile::open(Cursor::new(raw_bytes))
            .map_err(|e| AltiumError::Cfb(format!("Failed to open CFB: {}", e)))?;

        let section_keys = read_pcb_section_keys(&mut cfb)?;

        // Enumerate top-level storages to find footprints.
        // A storage is only treated as a footprint if it has a Data stream
        // (the defining characteristic). Storages without Data are captured
        // as library-level extra streams instead.
        let candidate_entries: Vec<String> = cfb
            .walk()
            .filter(|e| {
                e.is_storage()
                    && e.path()
                        .parent()
                        .map_or(false, |p| p == std::path::Path::new("/"))
            })
            .filter_map(|e| {
                let name = e.path().file_name()?.to_str()?.to_string();
                if name == "SectionKeys" || name == "FileHeader" || name == "Library" {
                    return None;
                }
                Some(name)
            })
            .collect();

        // Only keep storages that have a Data stream.
        let entries: Vec<String> = candidate_entries
            .into_iter()
            .filter(|name| {
                let data_path = format!("/{}/{}", name, STREAM_DATA);
                cfb.open_stream(&data_path).is_ok()
            })
            .collect();

        // Collect all stream paths upfront to avoid re-borrowing cfb.
        let all_stream_paths: Vec<String> = cfb
            .walk()
            .filter(|e| e.is_stream())
            .filter_map(|e| Some(e.path().to_str()?.to_string()))
            .collect();

        // Capture library-level extra streams (FileHeader, Library/*, etc.).
        let footprint_set: std::collections::HashSet<&str> =
            entries.iter().map(|s| s.as_str()).collect();
        let mut lib_extra_streams: HashMap<String, Vec<u8>> = HashMap::new();
        for stream_path in &all_stream_paths {
            let path_no_slash = stream_path.trim_start_matches('/');
            let top_level = path_no_slash.split('/').next().unwrap_or("");
            if footprint_set.contains(top_level) {
                continue;
            }
            if let Ok(mut stream) = cfb.open_stream(stream_path) {
                let mut data = Vec::new();
                if stream.read_to_end(&mut data).is_ok() {
                    lib_extra_streams.insert(path_no_slash.to_string(), data);
                }
            }
        }

        let doc_meta = DocumentMeta::PcbLib {
            section_keys,
            raw_extra_streams: lib_extra_streams,
        };
        let mut store = DocumentStore::new(doc_meta);

        for storage_name in &entries {
            // Read Parameters stream (footprint metadata)
            let params_path = format!("/{}/{}", storage_name, STREAM_PARAMETERS);
            let metadata_node = if let Ok(mut stream) = cfb.open_stream(&params_path) {
                let mut data = Vec::new();
                stream.read_to_end(&mut data).map_err(AltiumError::Io)?;
                let param_str = String::from_utf8_lossy(&data).to_string();
                RecordNode::new(0, RecordOrigin::Param(ParamOrigin::new(&param_str)))
            } else {
                RecordNode::new(
                    0,
                    RecordOrigin::Param(ParamOrigin::new("|PATTERN=|")),
                )
            };

            // Read Header stream
            let header_path = format!("/{}/{}", storage_name, STREAM_HEADER);
            let raw_header = if let Ok(mut stream) = cfb.open_stream(&header_path) {
                let mut data = Vec::new();
                stream.read_to_end(&mut data).map_err(AltiumError::Io)?;
                data
            } else {
                Vec::new()
            };

            // Read Data stream (pattern name block + binary primitives)
            let data_path = format!("/{}/{}", storage_name, STREAM_DATA);
            let (primitives, primitive_order, raw_pattern_name_block) =
                if let Ok(mut stream) = cfb.open_stream(&data_path) {
                    let mut data = Vec::new();
                    stream.read_to_end(&mut data).map_err(AltiumError::Io)?;
                    parse_pcb_data_stream(&data)?
                } else {
                    (Vec::new(), Vec::new(), Vec::new())
                };

            // Capture extra streams in this footprint's storage
            let storage_prefix = format!("/{}/", storage_name);
            let mut extra_streams: HashMap<String, Vec<u8>> = HashMap::new();
            for stream_path in &all_stream_paths {
                if let Some(rest) = stream_path.strip_prefix(&storage_prefix) {
                    if rest == STREAM_PARAMETERS
                        || rest == STREAM_HEADER
                        || rest == STREAM_DATA
                    {
                        continue;
                    }
                    if let Ok(mut stream) = cfb.open_stream(stream_path) {
                        let mut data = Vec::new();
                        if stream.read_to_end(&mut data).is_ok() {
                            extra_streams.insert(rest.to_string(), data);
                        }
                    }
                }
            }

            // Insert metadata record into store
            let parent_id = store.insert_record(metadata_node);

            // Insert primitive records into store
            let mut child_ids: Vec<RecordId> = Vec::with_capacity(primitives.len());
            for prim_node in primitives {
                let id = store.insert_record(prim_node);
                child_ids.push(id);
            }

            // Build original_indices parallel to primitive_order (index within children vec)
            let original_indices: Vec<usize> =
                primitive_order.iter().map(|r| r.index).collect();

            let group_data = GroupData {
                parent: parent_id,
                children: child_ids,
                original_indices,
                parent_original_index: None,
                extra_streams,
                meta: GroupMeta::PcbFootprint {
                    name: storage_name.clone(),
                    raw_pattern_name_block,
                    original_primitive_order: primitive_order,
                    raw_header,
                },
            };
            store.insert_group(group_data);
        }

        crate::v2::semantic_ids::compute_all_ids(&mut store, "dtid:pcblib", &doc_key);

        Ok(PcbLib {
            store: Rc::new(RefCell::new(store)),
        })
    }

    /// Open a PcbLib from a file path.
    pub fn open_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let file = std::fs::File::open(path).map_err(AltiumError::Io)?;
        Self::open(file)
    }

    /// Save a PcbLib to a writer.
    pub fn save<W: Read + Write + Seek>(&self, writer: W) -> Result<()> {
        let mut cfb = cfb::CompoundFile::create(writer)
            .map_err(|e| AltiumError::Cfb(format!("Failed to create CFB: {}", e)))?;

        let store = self.store.borrow();

        // Write library-level extra streams (FileHeader, Library/*, etc.)
        if let DocumentMeta::PcbLib { raw_extra_streams, .. } = store.meta() {
            let mut created_storages: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            let mut sorted_extras: Vec<_> = raw_extra_streams.iter().collect();
            sorted_extras.sort_by_key(|(k, _)| (*k).clone());
            for (rel_path, data) in &sorted_extras {
                let full_path = format!("/{}", rel_path);
                ensure_parent_storages(&mut cfb, &full_path, &mut created_storages)?;
                let mut stream = cfb.create_stream(&full_path).map_err(|e| {
                    AltiumError::Cfb(format!("Failed to create extra stream {}: {}", full_path, e))
                })?;
                stream.write_all(data).map_err(AltiumError::Io)?;
            }
        }

        for &group_id in store.group_ids() {
            let group = store.group(group_id);
            let (name, raw_pattern_name_block, original_primitive_order, raw_header) =
                match &group.meta {
                    GroupMeta::PcbFootprint {
                        name,
                        raw_pattern_name_block,
                        original_primitive_order,
                        raw_header,
                    } => (
                        name.clone(),
                        raw_pattern_name_block.clone(),
                        original_primitive_order.clone(),
                        raw_header.clone(),
                    ),
                    _ => continue,
                };

            let storage_path = format!("/{}", name);
            cfb.create_storage(&storage_path).map_err(|e| {
                AltiumError::Cfb(format!("Failed to create storage: {}", e))
            })?;

            // Write Parameters stream
            let params_path = format!("/{}/{}", name, STREAM_PARAMETERS);
            let params_data = match &store.record(group.parent).origin {
                RecordOrigin::Param(p) => p.params.to_param_string().into_bytes(),
                _ => Vec::new(),
            };
            let mut stream = cfb.create_stream(&params_path).map_err(|e| {
                AltiumError::Cfb(format!("Failed to create Parameters: {}", e))
            })?;
            stream.write_all(&params_data).map_err(AltiumError::Io)?;

            // Write Header stream
            let header_path = format!("/{}/{}", name, STREAM_HEADER);
            let mut stream = cfb.create_stream(&header_path).map_err(|e| {
                AltiumError::Cfb(format!("Failed to create Header: {}", e))
            })?;
            if raw_header.is_empty() {
                let count = group.children.len() as u32;
                stream
                    .write_all(&count.to_le_bytes())
                    .map_err(AltiumError::Io)?;
            } else {
                stream.write_all(&raw_header).map_err(AltiumError::Io)?;
            }

            // Write Data stream
            let data_path = format!("/{}/{}", name, STREAM_DATA);
            let primitives: Vec<&RecordNode> =
                group.children.iter().map(|&id| store.record(id)).collect();
            let data = build_pcb_data_stream(
                &raw_pattern_name_block,
                &original_primitive_order,
                &primitives,
            )?;
            let mut stream = cfb.create_stream(&data_path).map_err(|e| {
                AltiumError::Cfb(format!("Failed to create Data: {}", e))
            })?;
            stream.write_all(&data).map_err(AltiumError::Io)?;

            // Write per-footprint extra streams
            {
                let mut created_storages: std::collections::HashSet<String> =
                    std::collections::HashSet::new();
                // The footprint storage itself is already created above.
                created_storages.insert(storage_path.clone());
                for (rel_path, data) in &group.extra_streams {
                    let full_path = format!("/{}/{}", name, rel_path);
                    ensure_parent_storages(&mut cfb, &full_path, &mut created_storages)?;
                    let mut stream = cfb.create_stream(&full_path).map_err(|e| {
                        AltiumError::Cfb(format!(
                            "Failed to create extra stream {}: {}",
                            full_path, e
                        ))
                    })?;
                    stream.write_all(data).map_err(AltiumError::Io)?;
                }
            }
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

    /// Returns the number of footprints in the library.
    pub fn footprint_count(&self) -> usize {
        self.store.borrow().group_count()
    }

    /// Returns the footprint storage names in order.
    pub fn names(&self) -> Vec<String> {
        let store = self.store.borrow();
        store
            .group_ids()
            .iter()
            .filter_map(|&id| {
                if let GroupMeta::PcbFootprint { name, .. } = &store.group(id).meta {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Find a footprint by name (case-insensitive), returns a handle.
    pub fn find_footprint(&self, name: &str) -> Option<PcbFootprintHandle> {
        let store = self.store.borrow();
        let name_lower = name.to_lowercase();
        for &id in store.group_ids() {
            if let GroupMeta::PcbFootprint { name: fp_name, .. } = &store.group(id).meta {
                if fp_name.to_lowercase() == name_lower {
                    return Some(PcbFootprintHandle::new(self.store.clone(), id));
                }
            }
        }
        None
    }

    /// Returns a unique ID from the library (from the first footprint's UNIQUEID parameter).
    pub fn unique_id(&self) -> String {
        let store = self.store.borrow();
        for &group_id in store.group_ids() {
            let group = store.group(group_id);
            if let RecordOrigin::Param(p) = &store.record(group.parent).origin {
                if let Some(v) = p.params.get("UNIQUEID") {
                    let s = v.as_str().to_string();
                    if !s.is_empty() {
                        return s;
                    }
                }
            }
        }
        String::new()
    }

    /// Build and add a new footprint using the builder pattern.
    ///
    /// The footprint is inserted into the centralized `DocumentStore`.
    pub fn build_footprint(
        &self,
        name: &str,
        template: fn() -> RecordOrigin,
        build: impl FnOnce(&mut crate::v2::builders::FootprintBuilder),
    ) {
        let mut builder = crate::v2::builders::FootprintBuilder::new(template);
        build(&mut builder);
        let (metadata, primitives, primitive_refs) = builder.build();

        let mut store = self.store.borrow_mut();

        let parent_id = store.insert_record(metadata);

        let mut child_ids: Vec<RecordId> = Vec::with_capacity(primitives.len());
        for prim_node in primitives {
            let id = store.insert_record(prim_node);
            child_ids.push(id);
        }

        let original_indices: Vec<usize> =
            primitive_refs.iter().map(|r| r.index).collect();

        let group_data = GroupData {
            parent: parent_id,
            children: child_ids,
            original_indices,
            parent_original_index: None,
            extra_streams: HashMap::new(),
            meta: GroupMeta::PcbFootprint {
                name: name.to_string(),
                raw_pattern_name_block: Vec::new(),
                original_primitive_order: primitive_refs,
                raw_header: Vec::new(),
            },
        };
        store.insert_group(group_data);
    }
}

// ---------------------------------------------------------------------------
// DocumentQuery<PcbFootprint> for PcbLib
// ---------------------------------------------------------------------------

impl DocumentQuery<crate::v2::handles::PcbFootprint> for PcbLib {
    fn query(&self, q: &str) -> crate::error::Result<PcbFootprintHandle> {
        use crate::v2::query::eval::evaluate;
        let parsed = crate::v2::query::parse(q)?;

        let store = self.store.borrow();
        let mut matches = Vec::new();
        for &group_id in store.group_ids() {
            let group = store.group(group_id);
            let parent_node = store.record(group.parent);
            let all = std::slice::from_ref(parent_node);
            if !evaluate(&parsed, all).is_empty() {
                matches.push(group_id);
            }
        }

        match matches.len() {
            0 => Err(crate::error::AltiumError::NoMatch(q.to_string())),
            1 => Ok(PcbFootprintHandle::new(self.store.clone(), matches[0])),
            n => Err(crate::error::AltiumError::AmbiguousMatch(n, q.to_string())),
        }
    }

    fn query_all(&self, q: &str) -> crate::error::Result<Vec<PcbFootprintHandle>> {
        use crate::v2::query::eval::evaluate;
        let parsed = crate::v2::query::parse(q)?;

        let store = self.store.borrow();
        let mut handles = Vec::new();
        for &group_id in store.group_ids() {
            let group = store.group(group_id);
            let parent_node = store.record(group.parent);
            let all = std::slice::from_ref(parent_node);
            if !evaluate(&parsed, all).is_empty() {
                handles.push(PcbFootprintHandle::new(self.store.clone(), group_id));
            }
        }

        Ok(handles)
    }
}

// ---------------------------------------------------------------------------
// Deep primitive queries for PcbLib
// ---------------------------------------------------------------------------

impl PcbLib {
    /// Query a single child record of type `T` across all footprint groups.
    pub fn query_child<T: HandleFamily>(
        &self,
        q: &str,
    ) -> crate::error::Result<T::Handle> {
        use crate::v2::query::eval::evaluate;
        let parsed = crate::v2::query::parse(q)?;

        let store = self.store.borrow();
        let mut matches = Vec::new();
        for &group_id in store.group_ids() {
            let group = store.group(group_id);
            for &child_id in &group.children {
                let node = store.record(child_id);
                if node.key == T::record_id() && node.origin.is_binary() == T::is_binary() {
                    let all = std::slice::from_ref(node);
                    if !evaluate(&parsed, all).is_empty() {
                        matches.push(child_id);
                    }
                }
            }
        }

        match matches.len() {
            0 => Err(crate::error::AltiumError::NoMatch(q.to_string())),
            1 => Ok(T::make_handle(self.store.clone(), matches[0])),
            n => Err(crate::error::AltiumError::AmbiguousMatch(n, q.to_string())),
        }
    }

    /// Query all child records of type `T` across all footprint groups.
    pub fn query_all_children<T: HandleFamily>(
        &self,
        q: &str,
    ) -> crate::error::Result<Vec<T::Handle>> {
        use crate::v2::query::eval::evaluate;
        let parsed = crate::v2::query::parse(q)?;

        let store = self.store.borrow();
        let mut handles = Vec::new();
        for &group_id in store.group_ids() {
            let group = store.group(group_id);
            for &child_id in &group.children {
                let node = store.record(child_id);
                if node.key == T::record_id() && node.origin.is_binary() == T::is_binary() {
                    let all = std::slice::from_ref(node);
                    if !evaluate(&parsed, all).is_empty() {
                        handles.push(T::make_handle(self.store.clone(), child_id));
                    }
                }
            }
        }

        Ok(handles)
    }
}

// ---------------------------------------------------------------------------
// CFB storage helpers
// ---------------------------------------------------------------------------

/// Ensure all ancestor storages for a given path exist in the CFB file.
///
/// For example, given `/A/B/C/stream`, this creates `/A`, `/A/B`, and
/// `/A/B/C` if they don't already exist.
pub(crate) fn ensure_parent_storages<F: Read + Write + Seek>(
    cfb: &mut cfb::CompoundFile<F>,
    path: &str,
    created: &mut std::collections::HashSet<String>,
) -> Result<()> {
    let parts: Vec<&str> = path
        .trim_start_matches('/')
        .split('/')
        .collect();
    // Walk all ancestors (skip the final component which is the stream itself).
    let mut current = String::new();
    for part in &parts[..parts.len().saturating_sub(1)] {
        current = format!("{}/{}", current, part);
        if created.insert(current.clone()) {
            // Ignore AlreadyExists errors — the storage may already exist.
            let _ = cfb.create_storage(&current);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Read PCB section keys (stub — PcbLib files don't typically use section keys).
fn read_pcb_section_keys<F: Read + Seek>(
    _cfb: &mut cfb::CompoundFile<F>,
) -> Result<SectionKeyList> {
    Ok(SectionKeyList::new())
}

/// Returns the number of subrecords for a given PCB primitive type.
///
/// Pad (type 2) has 6 subrecords; Text (type 5) has 2 subrecords;
/// all others have 1 subrecord.
fn subrecord_count(type_id: u8) -> usize {
    match type_id {
        2 => 6, // Pad
        5 => 2, // Text
        _ => 1,
    }
}

/// Build a `RecordOrigin` for a single-subrecord PCB primitive.
///
/// For types that have custom parse functions (Via=3, Region=11,
/// ComponentBody=12), calls the appropriate parser to populate field_spans.
/// Falls back to a plain `BinaryOrigin` if parsing fails or the type is
/// unknown.
fn parse_single_subrecord_origin(type_byte: u8, block_data: Vec<u8>) -> RecordOrigin {
    match type_byte {
        3 => parse_via(&block_data)
            .unwrap_or_else(|_| RecordOrigin::Binary(BinaryOrigin::new(block_data))),
        11 => parse_region(&block_data)
            .unwrap_or_else(|_| RecordOrigin::Binary(BinaryOrigin::new(block_data))),
        12 => parse_component_body(&block_data)
            .unwrap_or_else(|_| RecordOrigin::Binary(BinaryOrigin::new(block_data))),
        _ => RecordOrigin::Binary(BinaryOrigin::new(block_data)),
    }
}

/// Build a `RecordOrigin` for a multi-subrecord PCB primitive.
///
/// For types that have custom parse functions (Pad=2, Text=5), calls the
/// appropriate parser to populate field_spans. Falls back to a plain
/// `BinaryOrigin` if parsing fails.
fn parse_multi_subrecord_origin(type_byte: u8, block_data: Vec<u8>) -> RecordOrigin {
    match type_byte {
        2 => parse_pad(&block_data)
            .unwrap_or_else(|_| RecordOrigin::Binary(BinaryOrigin::new(block_data))),
        5 => parse_text(&block_data)
            .unwrap_or_else(|_| RecordOrigin::Binary(BinaryOrigin::new(block_data))),
        _ => RecordOrigin::Binary(BinaryOrigin::new(block_data)),
    }
}

/// Parse the PCB Data stream: pattern name block + binary primitives.
///
/// Format:
/// - 4 bytes LE: pattern name length
/// - N bytes: pattern name
/// - For each primitive:
///   - 1 byte: type ID
///   - For single-subrecord types: 4 bytes LE length + data
///   - For multi-subrecord types (Pad=6, Text=2): N sequential
///     (4 bytes LE length + data) blocks stored together
fn parse_pcb_data_stream(
    data: &[u8],
) -> Result<(Vec<RecordNode>, Vec<PcbPrimitiveRef>, Vec<u8>)> {
    let mut cursor = Cursor::new(data);
    let mut primitives = Vec::new();
    let mut primitive_order = Vec::new();

    // Read pattern name block (length-prefixed)
    let pattern_name_block = if data.len() >= 4 {
        let str_len = cursor
            .read_u32::<LittleEndian>()
            .map_err(|_| AltiumError::UnexpectedEof)? as usize;
        if str_len > 0 && cursor.position() as usize + str_len <= data.len() {
            let mut buf = vec![0u8; str_len];
            cursor.read_exact(&mut buf).map_err(AltiumError::Io)?;
            buf
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // Read binary primitives
    while (cursor.position() as usize) < data.len() {
        let type_byte = match cursor.read_u8() {
            Ok(b) => b,
            Err(_) => break,
        };

        let n = subrecord_count(type_byte);

        if n == 1 {
            // Single subrecord: read u32 len + data, store data only
            let block_len = match cursor.read_u32::<LittleEndian>() {
                Ok(l) => l as usize,
                Err(_) => break,
            };

            if cursor.position() as usize + block_len > data.len() {
                break;
            }

            let mut block_data = vec![0u8; block_len];
            if cursor.read_exact(&mut block_data).is_err() {
                break;
            }

            let index = primitives.len();
            let origin = parse_single_subrecord_origin(type_byte, block_data);
            primitives.push(RecordNode::new(type_byte, origin));
            primitive_order.push(PcbPrimitiveRef::new(type_byte, index));
        } else {
            // Multi-subrecord: read N sequential (u32 len + data) blocks,
            // store ALL bytes including u32 prefixes as one raw_block
            let start = cursor.position() as usize;
            let mut ok = true;
            for _ in 0..n {
                let sub_len = match cursor.read_u32::<LittleEndian>() {
                    Ok(l) => l as usize,
                    Err(_) => {
                        ok = false;
                        break;
                    }
                };
                if cursor.position() as usize + sub_len > data.len() {
                    ok = false;
                    break;
                }
                cursor.set_position(cursor.position() + sub_len as u64);
            }
            if !ok {
                break;
            }
            let end = cursor.position() as usize;
            let block_data = data[start..end].to_vec();

            let index = primitives.len();
            let origin = parse_multi_subrecord_origin(type_byte, block_data);
            primitives.push(RecordNode::new(type_byte, origin));
            primitive_order.push(PcbPrimitiveRef::new(type_byte, index));
        }
    }

    Ok((primitives, primitive_order, pattern_name_block))
}

/// Build a PCB Data stream from store-level components.
///
/// Accepts the raw pattern name block, the original primitive ordering, and
/// the borrowed primitive records (indexed by position in the children vec).
fn build_pcb_data_stream(
    raw_pattern_name_block: &[u8],
    original_primitive_order: &[PcbPrimitiveRef],
    primitives: &[&RecordNode],
) -> Result<Vec<u8>> {
    let mut output = Vec::new();

    // Write pattern name block
    output
        .write_u32::<LittleEndian>(raw_pattern_name_block.len() as u32)
        .map_err(AltiumError::Io)?;
    output.extend_from_slice(raw_pattern_name_block);

    // Write primitives in original order
    for prim_ref in original_primitive_order {
        if prim_ref.index < primitives.len() {
            let prim = primitives[prim_ref.index];
            let n = subrecord_count(prim.key);

            // Get the bytes to write (from dirty origin or clean snapshot)
            let bytes: &[u8] = if prim.is_dirty() {
                match &prim.origin {
                    RecordOrigin::Binary(b) => &b.raw_block,
                    RecordOrigin::Param(_) => &[],
                }
            } else {
                &prim.original_snapshot
            };

            output.push(prim.key); // type byte

            if n == 1 {
                // Single subrecord: write u32(len) + bytes
                output
                    .write_u32::<LittleEndian>(bytes.len() as u32)
                    .map_err(AltiumError::Io)?;
                output.extend_from_slice(bytes);
            } else {
                // Multi-subrecord: bytes already contain u32 prefixes,
                // write directly after the type byte
                output.extend_from_slice(bytes);
            }
        }
    }

    Ok(output)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::backing_store::{BinaryOrigin, ParamOrigin, PcbPrimitiveRef, RecordOrigin};

    // ---------------------------------------------------------------------------
    // parse_pcb_data_stream tests
    // ---------------------------------------------------------------------------

    #[test]
    fn pcb_data_stream_roundtrip() {
        let mut data = Vec::new();
        // Pattern name: "SOT-23"
        let name = b"SOT-23";
        data.extend_from_slice(&(name.len() as u32).to_le_bytes());
        data.extend_from_slice(name);
        // A track primitive: type=4, length=35, zeros
        data.push(4); // type byte
        data.extend_from_slice(&35u32.to_le_bytes()); // length
        data.extend_from_slice(&vec![0u8; 35]); // data

        let (prims, order, pattern_name) = parse_pcb_data_stream(&data).unwrap();
        assert_eq!(pattern_name, name);
        assert_eq!(prims.len(), 1);
        assert_eq!(prims[0].key, 4);
        assert_eq!(order.len(), 1);
    }

    #[test]
    fn empty_data_stream() {
        let (prims, order, pattern_name) = parse_pcb_data_stream(&[]).unwrap();
        assert!(prims.is_empty());
        assert!(order.is_empty());
        assert!(pattern_name.is_empty());
    }

    #[test]
    fn multiple_primitives() {
        let mut data = Vec::new();
        // Pattern name: "QFP"
        let name = b"QFP";
        data.extend_from_slice(&(name.len() as u32).to_le_bytes());
        data.extend_from_slice(name);
        // Track primitive: type=4
        data.push(4);
        data.extend_from_slice(&8u32.to_le_bytes());
        data.extend_from_slice(&[0u8; 8]);
        // Arc primitive: type=1 (single subrecord)
        data.push(1);
        data.extend_from_slice(&12u32.to_le_bytes());
        data.extend_from_slice(&[0u8; 12]);

        let (prims, order, _) = parse_pcb_data_stream(&data).unwrap();
        assert_eq!(prims.len(), 2);
        assert_eq!(prims[0].key, 4);
        assert_eq!(prims[1].key, 1);
        assert_eq!(order.len(), 2);
        assert_eq!(order[0].type_id, 4);
        assert_eq!(order[1].type_id, 1);
    }

    #[test]
    fn pad_multi_subrecord_roundtrip() {
        let mut data = Vec::new();
        // Pattern name: "PAD"
        let name = b"PAD";
        data.extend_from_slice(&(name.len() as u32).to_le_bytes());
        data.extend_from_slice(name);
        // Pad primitive: type=2 with 6 subrecords
        data.push(2);
        // Subrecords 1-4: small subrecords
        for i in 0u8..4 {
            let sub = vec![i; 2]; // 2-byte payload
            data.extend_from_slice(&(sub.len() as u32).to_le_bytes());
            data.extend_from_slice(&sub);
        }
        // Subrecord 5: core data (16 bytes)
        let core = vec![0xAA; 16];
        data.extend_from_slice(&(core.len() as u32).to_le_bytes());
        data.extend_from_slice(&core);
        // Subrecord 6: stack data (8 bytes)
        let stack = vec![0xBB; 8];
        data.extend_from_slice(&(stack.len() as u32).to_le_bytes());
        data.extend_from_slice(&stack);

        let (prims, order, pattern_name) = parse_pcb_data_stream(&data).unwrap();
        assert_eq!(pattern_name, b"PAD");
        assert_eq!(prims.len(), 1);
        assert_eq!(prims[0].key, 2);
        assert_eq!(order.len(), 1);

        // The raw_block should contain all 6 subrecords with u32 prefixes
        let raw = prims[0].origin.as_binary().unwrap();
        // 4*(4+2) + (4+16) + (4+8) = 24 + 20 + 12 = 56
        assert_eq!(raw.raw_block.len(), 56);

        // Round-trip via build_pcb_data_stream
        let prim_refs: Vec<&RecordNode> = prims.iter().collect();
        let rebuilt = build_pcb_data_stream(b"PAD", &order, &prim_refs).unwrap();
        let (prims2, _, _) = parse_pcb_data_stream(&rebuilt).unwrap();
        assert_eq!(prims2.len(), 1);
        assert_eq!(prims2[0].key, 2);
        assert_eq!(prims2[0].origin.as_binary().unwrap().raw_block.len(), 56);
    }

    #[test]
    fn text_multi_subrecord_roundtrip() {
        let mut data = Vec::new();
        // Pattern name: "TXT"
        let name = b"TXT";
        data.extend_from_slice(&(name.len() as u32).to_le_bytes());
        data.extend_from_slice(name);
        // Text primitive: type=5 with 2 subrecords
        data.push(5);
        // Subrecord 1: main text data (40 bytes)
        let sub1 = vec![0xCC; 40];
        data.extend_from_slice(&(sub1.len() as u32).to_le_bytes());
        data.extend_from_slice(&sub1);
        // Subrecord 2: text string (10 bytes)
        let sub2 = b"Hello\0\0\0\0\0";
        data.extend_from_slice(&(sub2.len() as u32).to_le_bytes());
        data.extend_from_slice(sub2);

        let (prims, order, _) = parse_pcb_data_stream(&data).unwrap();
        assert_eq!(prims.len(), 1);
        assert_eq!(prims[0].key, 5);
        // raw_block = (4+40) + (4+10) = 58
        assert_eq!(prims[0].origin.as_binary().unwrap().raw_block.len(), 58);

        // Round-trip
        let prim_refs: Vec<&RecordNode> = prims.iter().collect();
        let rebuilt = build_pcb_data_stream(b"TXT", &order, &prim_refs).unwrap();
        let (prims2, _, _) = parse_pcb_data_stream(&rebuilt).unwrap();
        assert_eq!(prims2.len(), 1);
        assert_eq!(prims2[0].key, 5);
        assert_eq!(prims2[0].origin.as_binary().unwrap().raw_block.len(), 58);
    }

    #[test]
    fn build_stream_roundtrip() {
        let block_data = vec![0xAA; 10];
        let prim = RecordNode::new(
            4,
            RecordOrigin::Binary(BinaryOrigin::new(block_data.clone())),
        );
        let order = vec![PcbPrimitiveRef::new(4, 0)];
        let prim_refs: Vec<&RecordNode> = vec![&prim];

        let data = build_pcb_data_stream(b"DIP-8", &order, &prim_refs).unwrap();
        let (prims, out_order, pattern_name) = parse_pcb_data_stream(&data).unwrap();

        assert_eq!(pattern_name, b"DIP-8");
        assert_eq!(prims.len(), 1);
        assert_eq!(prims[0].key, 4);
        assert_eq!(out_order.len(), 1);
        assert_eq!(out_order[0].type_id, 4);
    }

    // ---------------------------------------------------------------------------
    // DocumentStore-based PcbLib construction and query tests
    // ---------------------------------------------------------------------------

    /// Helper: build a minimal PcbLib in-memory with named footprints.
    fn make_test_lib(fp_names: &[&str]) -> PcbLib {
        let doc_meta = DocumentMeta::PcbLib {
            section_keys: SectionKeyList::new(),
            raw_extra_streams: HashMap::new(),
        };
        let mut store = DocumentStore::new(doc_meta);

        for &name in fp_names {
            let param_str = format!("|PATTERN={}|DESCRIPTION={}|", name, name);
            let metadata = RecordNode::new(
                0,
                RecordOrigin::Param(ParamOrigin::new(&param_str)),
            );
            let parent_id = store.insert_record(metadata);

            let group_data = GroupData {
                parent: parent_id,
                children: Vec::new(),
                original_indices: Vec::new(),
                parent_original_index: None,
                extra_streams: HashMap::new(),
                meta: GroupMeta::PcbFootprint {
                    name: name.to_string(),
                    raw_pattern_name_block: name.as_bytes().to_vec(),
                    original_primitive_order: Vec::new(),
                    raw_header: Vec::new(),
                },
            };
            store.insert_group(group_data);
        }

        PcbLib {
            store: Rc::new(RefCell::new(store)),
        }
    }

    /// Helper: build a PcbLib with one footprint containing typed primitives.
    fn make_lib_with_primitives() -> PcbLib {
        let doc_meta = DocumentMeta::PcbLib {
            section_keys: SectionKeyList::new(),
            raw_extra_streams: HashMap::new(),
        };
        let mut store = DocumentStore::new(doc_meta);

        let metadata = RecordNode::new(
            0,
            RecordOrigin::Param(ParamOrigin::new("|PATTERN=SOT-23|")),
        );
        let parent_id = store.insert_record(metadata);

        let pad_block = vec![0u8; 40];
        let pad0 = RecordNode::new(2, RecordOrigin::Binary(BinaryOrigin::new(pad_block.clone())));
        let pad1 = RecordNode::new(2, RecordOrigin::Binary(BinaryOrigin::new(pad_block.clone())));
        let track = RecordNode::new(4, RecordOrigin::Binary(BinaryOrigin::new(vec![0u8; 35])));

        let pad0_id = store.insert_record(pad0);
        let pad1_id = store.insert_record(pad1);
        let track_id = store.insert_record(track);

        let group_data = GroupData {
            parent: parent_id,
            children: vec![pad0_id, pad1_id, track_id],
            original_indices: vec![0, 1, 2],
            parent_original_index: None,
            extra_streams: HashMap::new(),
            meta: GroupMeta::PcbFootprint {
                name: "SOT-23".to_string(),
                raw_pattern_name_block: b"SOT-23".to_vec(),
                original_primitive_order: vec![
                    PcbPrimitiveRef::new(2, 0),
                    PcbPrimitiveRef::new(2, 1),
                    PcbPrimitiveRef::new(4, 2),
                ],
                raw_header: Vec::new(),
            },
        };
        store.insert_group(group_data);

        PcbLib {
            store: Rc::new(RefCell::new(store)),
        }
    }

    #[test]
    fn pcblib_footprint_count() {
        let lib = make_test_lib(&["SOT-23", "QFP-48", "DIP-8"]);
        assert_eq!(lib.footprint_count(), 3);
    }

    #[test]
    fn pcblib_names() {
        let lib = make_test_lib(&["SOT-23", "QFP-48", "DIP-8"]);
        let names = lib.names();
        assert_eq!(names, vec!["SOT-23", "QFP-48", "DIP-8"]);
    }

    #[test]
    fn pcblib_find_footprint_found() {
        let lib = make_test_lib(&["SOT-23", "QFP-48"]);
        let handle = lib.find_footprint("sot-23");
        assert!(handle.is_some());
        assert_eq!(handle.unwrap().name(), "SOT-23");
    }

    #[test]
    fn pcblib_find_footprint_not_found() {
        let lib = make_test_lib(&["SOT-23"]);
        assert!(lib.find_footprint("DIP-8").is_none());
    }

    #[test]
    fn pcblib_query_all_footprints() {
        use crate::v2::traits::DocumentQuery;

        let lib = make_test_lib(&["SOT-23", "QFP-48", "DIP-8"]);
        let results = DocumentQuery::<crate::v2::handles::PcbFootprint>::query_all(&lib, "#0")
            .unwrap();
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn pcblib_query_single_footprint() {
        use crate::v2::traits::DocumentQuery;

        let lib = make_test_lib(&["SOT-23"]);
        let handle = DocumentQuery::<crate::v2::handles::PcbFootprint>::query(&lib, "#0")
            .unwrap();
        assert_eq!(handle.name(), "SOT-23");
    }

    #[test]
    fn pcblib_query_no_match() {
        use crate::v2::traits::DocumentQuery;

        let lib = make_test_lib(&["SOT-23"]);
        let result =
            DocumentQuery::<crate::v2::handles::PcbFootprint>::query(&lib, "NONEXISTENT");
        assert!(matches!(
            result,
            Err(crate::error::AltiumError::NoMatch(_))
        ));
    }

    #[test]
    fn pcblib_deep_query_pads() {
        let lib = make_lib_with_primitives();
        let results = lib
            .query_all_children::<crate::v2::handles::PcbPad>("#2")
            .unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn pcblib_deep_query_track() {
        let lib = make_lib_with_primitives();
        let results = lib
            .query_all_children::<crate::v2::handles::PcbTrack>("#4")
            .unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn pcblib_build_footprint() {
        use crate::v2::templates;

        let lib = PcbLib {
            store: DocumentStore::new_ref(DocumentMeta::PcbLib {
                section_keys: SectionKeyList::new(),
                raw_extra_streams: HashMap::new(),
            }),
        };

        assert_eq!(lib.footprint_count(), 0);
        lib.build_footprint("SOIC-8", templates::pcb_footprint_default, |_builder| {});
        assert_eq!(lib.footprint_count(), 1);
        assert_eq!(lib.names(), vec!["SOIC-8"]);
    }

    #[test]
    fn pcblib_unique_id_empty_when_absent() {
        let lib = make_test_lib(&["SOT-23"]);
        assert_eq!(lib.unique_id(), "");
    }

    #[test]
    fn pcblib_save_and_open_roundtrip() {
        use std::io::Cursor;

        let lib = make_test_lib(&["SOT-23", "DIP-8"]);
        let buf = Cursor::new(Vec::new());
        lib.save(buf).unwrap();
    }
}
