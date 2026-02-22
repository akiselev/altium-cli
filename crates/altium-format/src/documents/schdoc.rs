//! SchDoc document I/O using the v2 DocumentStore-based architecture.
//!
//! A SchDoc file is a CFB compound file with a single `/FileHeader` stream
//! containing all records as a flat length-prefixed sequence. Records are
//! grouped by OWNERINDEX: component records (RECORD=1) own child records
//! that reference them by index.

use std::io::{Read, Seek, Write};

use crate::backing_store::{ParamOrigin, RecordNode, RecordOrigin};
use crate::documents::schdoc_streams::{
    SchDocAdditionalStreamMeta, SchDocFileHeaderStreamMeta, SchDocRawBlock,
    SchDocStorageStreamMeta, parse_additional_meta_and_blocks, parse_file_header_meta_and_blocks,
    parse_storage_meta,
};
use crate::error::{AltiumError, Result};
use crate::ids::{GroupId, RecordId};
use crate::parameters::ParameterCollection;
use crate::records::{SchBlanketRecord, is_supported_sch_record_id};
use crate::store::{DocRef, DocumentMeta, DocumentStore, GroupData, GroupMeta};

const STREAM_FILE_HEADER: &str = "FileHeader";
const STREAM_ADDITIONAL: &str = "Additional";
const STREAM_STORAGE: &str = "Storage";

struct SchDocRootStreamPaths {
    file_header: String,
    additional: Option<String>,
    storage: String,
}

/// A parsed SchDoc document using the v2 DocumentStore architecture.
///
/// Records are grouped by OWNERINDEX. Component records (RECORD=1) form
/// groups with their children. Records that don't belong to any component
/// are stored as orphans in the shared store.
pub struct SchDoc {
    store: DocRef,
}

impl SchDoc {
    /// Create a new empty SchDoc document.
    pub fn new_empty() -> Self {
        let mut store = DocumentStore::new(DocumentMeta::SchDoc {
            file_header_meta: SchDocFileHeaderStreamMeta::default(),
            additional_meta: None,
            storage_meta: SchDocStorageStreamMeta::default(),
        });
        store.set_semantic_context("dtid:schdoc", "");
        Self {
            store: std::rc::Rc::new(std::cell::RefCell::new(store)),
        }
    }

    /// Returns typed `/FileHeader` stream metadata.
    pub fn file_header_meta(&self) -> SchDocFileHeaderStreamMeta {
        let store = self.store.borrow();
        match store.meta() {
            DocumentMeta::SchDoc {
                file_header_meta, ..
            } => file_header_meta.clone(),
            _ => SchDocFileHeaderStreamMeta::default(),
        }
    }

    /// Replace typed `/FileHeader` stream metadata.
    pub fn set_file_header_meta(&self, meta: SchDocFileHeaderStreamMeta) -> Result<()> {
        let mut store = self.store.borrow_mut();
        match store.meta_mut() {
            DocumentMeta::SchDoc {
                file_header_meta, ..
            } => {
                *file_header_meta = meta;
                Ok(())
            }
            other => Err(AltiumError::TypeMismatch(format!(
                "expected SchDoc, got {}",
                other.variant_name()
            ))),
        }
    }

    /// Returns typed `/Additional` stream metadata if the stream exists.
    pub fn additional_meta(&self) -> Option<SchDocAdditionalStreamMeta> {
        let store = self.store.borrow();
        match store.meta() {
            DocumentMeta::SchDoc {
                additional_meta, ..
            } => additional_meta.clone(),
            _ => None,
        }
    }

    /// Replace typed `/Additional` stream metadata.
    pub fn set_additional_meta(&self, meta: Option<SchDocAdditionalStreamMeta>) -> Result<()> {
        let mut store = self.store.borrow_mut();
        match store.meta_mut() {
            DocumentMeta::SchDoc {
                additional_meta, ..
            } => {
                *additional_meta = meta;
                Ok(())
            }
            other => Err(AltiumError::TypeMismatch(format!(
                "expected SchDoc, got {}",
                other.variant_name()
            ))),
        }
    }

    /// Returns typed `/Storage` stream metadata.
    pub fn storage_meta(&self) -> SchDocStorageStreamMeta {
        let store = self.store.borrow();
        match store.meta() {
            DocumentMeta::SchDoc { storage_meta, .. } => storage_meta.clone(),
            _ => SchDocStorageStreamMeta::default(),
        }
    }

    /// Replace typed `/Storage` stream metadata.
    pub fn set_storage_meta(&self, meta: SchDocStorageStreamMeta) -> Result<()> {
        let mut store = self.store.borrow_mut();
        match store.meta_mut() {
            DocumentMeta::SchDoc { storage_meta, .. } => {
                *storage_meta = meta;
                Ok(())
            }
            other => Err(AltiumError::TypeMismatch(format!(
                "expected SchDoc, got {}",
                other.variant_name()
            ))),
        }
    }

    /// Returns the stable document-level semantic ID, if computed.
    pub fn document_id(&self) -> Option<crate::semantic_ids::SemanticId> {
        let mut store = self.store.borrow_mut();
        store.ensure_semantic_ids();
        store.document_id().cloned()
    }

    /// Open a SchDoc from a reader.
    pub fn open<R: Read + Seek>(reader: R) -> Result<Self> {
        let mut cfb = cfb::CompoundFile::open(reader)
            .map_err(|e| AltiumError::Cfb(format!("Failed to open CFB: {}", e)))?;

        let stream_paths = collect_schdoc_root_stream_paths(&cfb)?;
        let file_header_raw = read_stream_bytes(&mut cfb, &stream_paths.file_header)?;
        let additional_raw = match stream_paths.additional.as_ref() {
            Some(path) => Some(read_stream_bytes(&mut cfb, path)?),
            None => None,
        };
        let storage_raw = read_stream_bytes(&mut cfb, &stream_paths.storage)?;

        let (file_header_meta, file_header_blocks) =
            parse_file_header_meta_and_blocks(&file_header_raw)?;
        let (additional_meta, additional_blocks) = if let Some(raw) = additional_raw.as_ref() {
            let (meta, blocks) = parse_additional_meta_and_blocks(raw)?;
            (Some(meta), Some(blocks))
        } else {
            (None, None)
        };
        let storage_meta = parse_storage_meta(&storage_raw)?;

        let meta = DocumentMeta::SchDoc {
            file_header_meta,
            additional_meta,
            storage_meta,
        };
        let mut doc_store = DocumentStore::new(meta);

        let mut key_bytes = file_header_raw.clone();
        if let Some(additional) = &additional_raw {
            key_bytes.extend_from_slice(additional);
        }
        let doc_key = crate::semantic_ids::blake3_content_hash(&key_bytes);

        let mut records = parse_record_blocks(&file_header_blocks, STREAM_FILE_HEADER)?;
        if let Some(blocks) = additional_blocks {
            records.extend(parse_record_blocks(&blocks, STREAM_ADDITIONAL)?);
        }
        group_by_owner_index(&mut doc_store, records);

        crate::semantic_ids::compute_all_ids(&mut doc_store, "dtid:schdoc", &doc_key);

        Ok(SchDoc {
            store: std::rc::Rc::new(std::cell::RefCell::new(doc_store)),
        })
    }

    /// Open a SchDoc from a file path.
    pub fn open_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let file = std::fs::File::open(path).map_err(AltiumError::Io)?;
        Self::open(file)
    }

    /// Save a SchDoc to a writer.
    pub fn save<W: Read + Write + Seek>(&self, writer: W) -> Result<()> {
        {
            let mut store = self.store.borrow_mut();
            store.ensure_semantic_ids();
        }
        let mut cfb = cfb::CompoundFile::create_with_version(cfb::Version::V3, writer)
            .map_err(|e| AltiumError::Cfb(format!("Failed to create CFB: {}", e)))?;

        let serialized = flatten_to_streams(self)?;

        let mut stream = cfb
            .create_stream(format!("/{}", STREAM_FILE_HEADER))
            .map_err(|e| AltiumError::Cfb(format!("Failed to create FileHeader: {}", e)))?;
        stream
            .write_all(&serialized.file_header)
            .map_err(AltiumError::Io)?;

        if !serialized.additional.is_empty() {
            let mut stream = cfb
                .create_stream(format!("/{}", STREAM_ADDITIONAL))
                .map_err(|e| AltiumError::Cfb(format!("Failed to create Additional: {}", e)))?;
            stream
                .write_all(&serialized.additional)
                .map_err(AltiumError::Io)?;
        }

        let mut storage_stream = cfb
            .create_stream(format!("/{}", STREAM_STORAGE))
            .map_err(|e| AltiumError::Cfb(format!("Failed to create Storage: {}", e)))?;
        storage_stream
            .write_all(&serialized.storage)
            .map_err(AltiumError::Io)?;

        cfb.flush()
            .map_err(|e| AltiumError::Cfb(format!("CFB flush: {}", e)))?;
        Ok(())
    }

    /// Save to a file path.
    pub fn save_file(&self, path: impl AsRef<std::path::Path>) -> Result<()> {
        let file = std::fs::File::create(path).map_err(AltiumError::Io)?;
        self.save(file)
    }

    /// Returns the number of components (groups) in the document.
    pub fn component_count(&self) -> usize {
        self.store.borrow().group_count()
    }

    /// Returns component handles in stable group order.
    pub fn components(&self) -> Vec<crate::handles::SchComponentHandle> {
        let store = self.store.borrow();
        store
            .group_ids()
            .iter()
            .map(|&gid| crate::handles::SchComponentHandle::new(self.store.clone(), gid))
            .collect()
    }

    /// Query for a single schematic component handle.
    pub fn query_component(&self, q: &str) -> Result<crate::handles::SchComponentHandle> {
        <Self as crate::traits::DocumentQuery<crate::handles::SchComponent>>::query(self, q)
    }

    /// Query for all schematic component handles matching `q`.
    pub fn query_all_components(&self, q: &str) -> Result<Vec<crate::handles::SchComponentHandle>> {
        <Self as crate::traits::DocumentQuery<crate::handles::SchComponent>>::query_all(self, q)
    }

    /// Query for a single record handle of type `T` across component parents,
    /// children, and orphan records.
    pub fn query<T: crate::traits::HandleFamily>(&self, q: &str) -> Result<T::Handle> {
        let matches = self.query_all::<T>(q)?;
        match matches.len() {
            0 => Err(AltiumError::NoMatch(q.to_string())),
            1 => Ok(matches.into_iter().next().expect("single element")),
            n => Err(AltiumError::AmbiguousMatch(n, q.to_string())),
        }
    }

    /// Query for all record handles of type `T` across component parents,
    /// children, and orphan records.
    pub fn query_all<T: crate::traits::HandleFamily>(&self, q: &str) -> Result<Vec<T::Handle>> {
        use crate::query::eval::evaluate;
        let parsed = crate::query::parse(q)?;

        let store = self.store.borrow();
        let mut handles = Vec::new();

        for &gid in store.group_ids() {
            let group = store.group(gid);
            let parent_id = group.parent_id();
            let parent = store.record(parent_id);
            if parent.key == T::record_id() && parent.origin.is_binary() == T::is_binary() {
                let all = std::slice::from_ref(parent);
                if !evaluate(&parsed, all).is_empty() {
                    handles.push(T::try_make_handle(self.store.clone(), parent_id)?);
                }
            }
            for &cid in group.child_ids() {
                let child = store.record(cid);
                if child.key == T::record_id() && child.origin.is_binary() == T::is_binary() {
                    let all = std::slice::from_ref(child);
                    if !evaluate(&parsed, all).is_empty() {
                        handles.push(T::try_make_handle(self.store.clone(), cid)?);
                    }
                }
            }
        }

        for &oid in store.orphan_ids() {
            let orphan = store.record(oid);
            if orphan.key == T::record_id() && orphan.origin.is_binary() == T::is_binary() {
                let all = std::slice::from_ref(orphan);
                if !evaluate(&parsed, all).is_empty() {
                    handles.push(T::try_make_handle(self.store.clone(), oid)?);
                }
            }
        }

        Ok(handles)
    }

    /// Returns orphan records in stable flat-stream order as `(record_id, record_ref)`.
    pub fn orphan_records(&self) -> Vec<(u8, RecordId)> {
        let store = self.store.borrow();
        store
            .orphan_ids()
            .iter()
            .map(|&rid| (store.record(rid).key, rid))
            .collect()
    }

    /// Add a new orphan record using high-level typed record APIs only.
    pub fn add_orphan_record<R>(&self, record: R) -> RecordId
    where
        R: crate::traits::FromOrigin + crate::traits::RecordType,
    {
        let mut store = self.store.borrow_mut();
        let mut node = RecordNode::new(R::RECORD_ID, record.into_origin());
        node.mark_dirty();
        let rid = store.insert_record(node);
        store.orphan_records.push(rid);
        store.orphan_original_indices.push(usize::MAX);
        store.mark_semantic_ids_dirty();
        rid
    }

    /// Build and add a new component using the high-level component builder.
    pub fn build_component(
        &self,
        template: fn() -> RecordOrigin,
        build: impl FnOnce(&mut crate::builders::ComponentBuilder),
    ) -> crate::handles::SchComponentHandle {
        let mut builder = crate::builders::ComponentBuilder::new(template);
        build(&mut builder);

        let (component, children) = builder.build();

        let mut store = self.store.borrow_mut();
        let parent_id = store.insert_record(component);
        let mut child_ids = Vec::with_capacity(children.len());
        for child in children {
            let id = store.insert_record(child);
            child_ids.push(id);
        }
        let child_len = child_ids.len();

        let group_id = store.insert_group(GroupData {
            parent: parent_id,
            children: child_ids,
            original_indices: vec![usize::MAX; child_len],
            parent_original_index: None,
            meta: GroupMeta::SchDocComponent,
        });
        store.mark_semantic_ids_dirty();
        crate::handles::SchComponentHandle::new(self.store.clone(), group_id)
    }

    /// Count all records of a given type across groups and orphans.
    pub fn count_record_type(&self, record_id: u8) -> usize {
        let store = self.store.borrow();
        let mut count = 0;

        for &gid in store.group_ids() {
            let group = store.group(gid);
            if store.record(group.parent_id()).key == record_id {
                count += 1;
            }
            count += group
                .child_ids()
                .iter()
                .filter(|&&id| store.record(id).key == record_id)
                .count();
        }

        count += store
            .orphan_ids()
            .iter()
            .filter(|&&id| store.record(id).key == record_id)
            .count();

        count
    }

    /// Returns the sheet record (RECORD=31) if present.
    pub fn sheet_record(&self) -> Option<crate::records::SchSheetRecord> {
        let id = crate::records::SchSheetRecord::RECORD_ID;
        let store = self.store.borrow();
        store
            .orphan_ids()
            .iter()
            .find(|&&rid| store.record(rid).key == id)
            .map(|&rid| {
                crate::records::SchSheetRecord::from_origin(store.record(rid).origin.clone())
            })
    }

    /// Construct a typed handle for a record in this document's store.
    pub fn handle_for<H: crate::traits::HandleFamily>(&self, rid: RecordId) -> Result<H::Handle> {
        H::try_make_handle(self.store.clone(), rid)
    }

    /// Returns the number of orphan records (records not owned by any component).
    pub fn orphan_count(&self) -> usize {
        self.store.borrow().orphan_ids().len()
    }

    /// Iterate all records of a given type across group children and orphans.
    ///
    /// Passes a cloned `RecordNode` for each matching record.
    pub fn for_each_record_of_type(&self, record_id: u8, mut f: impl FnMut(&RecordNode)) {
        let store = self.store.borrow();

        for &gid in store.group_ids() {
            let group = store.group(gid);
            for &cid in group.child_ids() {
                let node = store.record(cid);
                if node.key == record_id {
                    f(node);
                }
            }
        }

        for &oid in store.orphan_ids() {
            let node = store.record(oid);
            if node.key == record_id {
                f(node);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// DocumentQuery<SchComponent> for SchDoc
// ---------------------------------------------------------------------------

impl crate::traits::DocumentQuery<crate::handles::SchComponent> for SchDoc {
    fn query(&self, q: &str) -> crate::error::Result<crate::handles::SchComponentHandle> {
        use crate::query::eval::evaluate;
        let parsed = crate::query::parse(q)?;

        let store = self.store.borrow();
        let group_ids: Vec<GroupId> = store.group_ids().to_vec();

        let eval_nodes: Vec<RecordNode> = group_ids
            .iter()
            .map(|&gid| store.record(store.group(gid).parent_id()).clone())
            .collect();

        let matching = evaluate(&parsed, &eval_nodes);
        drop(store);

        match matching.len() {
            0 => Err(crate::error::AltiumError::NoMatch(q.to_string())),
            1 => Ok(crate::handles::SchComponentHandle::new(
                self.store.clone(),
                group_ids[matching[0]],
            )),
            n => Err(crate::error::AltiumError::AmbiguousMatch(n, q.to_string())),
        }
    }

    fn query_all(&self, q: &str) -> crate::error::Result<Vec<crate::handles::SchComponentHandle>> {
        use crate::query::eval::evaluate;
        let parsed = crate::query::parse(q)?;

        let store = self.store.borrow();
        let group_ids: Vec<GroupId> = store.group_ids().to_vec();

        let eval_nodes: Vec<RecordNode> = group_ids
            .iter()
            .map(|&gid| store.record(store.group(gid).parent_id()).clone())
            .collect();

        let indices = evaluate(&parsed, &eval_nodes);
        drop(store);

        let handles = indices
            .into_iter()
            .map(|i| crate::handles::SchComponentHandle::new(self.store.clone(), group_ids[i]))
            .collect();

        Ok(handles)
    }
}

// ---------------------------------------------------------------------------
// Deep child queries for SchDoc
// ---------------------------------------------------------------------------

impl SchDoc {
    /// Query a single child record of type `T` across all component groups.
    ///
    /// Returns `NoMatch` if no children of type `T` match, `AmbiguousMatch`
    /// if more than one matches.
    pub fn query_child<T: crate::traits::HandleFamily>(
        &self,
        q: &str,
    ) -> crate::error::Result<T::Handle> {
        use crate::query::eval::evaluate;
        let parsed = crate::query::parse(q)?;

        let store = self.store.borrow();
        let mut matches: Vec<RecordId> = Vec::new();

        for &gid in store.group_ids() {
            let group = store.group(gid);
            for &cid in group.child_ids() {
                let node = store.record(cid);
                if node.key == T::record_id() && node.origin.is_binary() == T::is_binary() {
                    let all = std::slice::from_ref(node);
                    if !evaluate(&parsed, all).is_empty() {
                        matches.push(cid);
                    }
                }
            }
        }
        drop(store);

        match matches.len() {
            0 => Err(crate::error::AltiumError::NoMatch(q.to_string())),
            1 => T::try_make_handle(self.store.clone(), matches[0]),
            n => Err(crate::error::AltiumError::AmbiguousMatch(n, q.to_string())),
        }
    }

    /// Query all child records of type `T` across all component groups.
    pub fn query_all_children<T: crate::traits::HandleFamily>(
        &self,
        q: &str,
    ) -> crate::error::Result<Vec<T::Handle>> {
        use crate::query::eval::evaluate;
        let parsed = crate::query::parse(q)?;

        let store = self.store.borrow();
        let mut matches: Vec<RecordId> = Vec::new();

        for &gid in store.group_ids() {
            let group = store.group(gid);
            for &cid in group.child_ids() {
                let node = store.record(cid);
                if node.key == T::record_id() && node.origin.is_binary() == T::is_binary() {
                    let all = std::slice::from_ref(node);
                    if !evaluate(&parsed, all).is_empty() {
                        matches.push(cid);
                    }
                }
            }
        }
        drop(store);

        let handles = matches
            .into_iter()
            .map(|id| T::try_make_handle(self.store.clone(), id))
            .collect::<crate::error::Result<Vec<_>>>()?;

        Ok(handles)
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn collect_schdoc_root_stream_paths<R: Read + Seek>(
    cfb: &cfb::CompoundFile<R>,
) -> Result<SchDocRootStreamPaths> {
    let mut file_header: Option<String> = None;
    let mut additional: Option<String> = None;
    let mut storage: Option<String> = None;

    for entry in cfb.walk().filter(|e| e.is_stream()) {
        let path = entry
            .path()
            .to_str()
            .ok_or_else(|| AltiumError::Parse("schdoc contains non-UTF8 stream path".to_string()))?
            .to_string();
        let rel = path.trim_start_matches('/');
        let mut parts = rel.split('/');
        let root = parts.next().unwrap_or("");

        if parts.next().is_some() {
            return Err(AltiumError::Parse(format!(
                "schdoc contains nested stream '{}'",
                path
            )));
        }

        if root.eq_ignore_ascii_case(STREAM_FILE_HEADER) {
            if file_header.is_some() {
                return Err(AltiumError::Parse(format!(
                    "schdoc contains duplicate '{}' stream",
                    STREAM_FILE_HEADER
                )));
            }
            file_header = Some(path);
            continue;
        }
        if root.eq_ignore_ascii_case(STREAM_ADDITIONAL) {
            if additional.is_some() {
                return Err(AltiumError::Parse(format!(
                    "schdoc contains duplicate '{}' stream",
                    STREAM_ADDITIONAL
                )));
            }
            additional = Some(path);
            continue;
        }
        if root.eq_ignore_ascii_case(STREAM_STORAGE) {
            if storage.is_some() {
                return Err(AltiumError::Parse(format!(
                    "schdoc contains duplicate '{}' stream",
                    STREAM_STORAGE
                )));
            }
            storage = Some(path);
            continue;
        }

        return Err(AltiumError::Parse(format!(
            "schdoc contains unimplemented stream '{}'",
            path
        )));
    }

    let file_header = file_header.ok_or_else(|| {
        AltiumError::Parse(format!(
            "schdoc missing required '{}' stream",
            STREAM_FILE_HEADER
        ))
    })?;
    let storage = storage.ok_or_else(|| {
        AltiumError::Parse(format!(
            "schdoc missing required '{}' stream",
            STREAM_STORAGE
        ))
    })?;

    Ok(SchDocRootStreamPaths {
        file_header,
        additional,
        storage,
    })
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

/// Parse record blocks into indexed `(flat_index, RecordNode)` pairs.
///
/// Block index is preserved, so the first non-record header block still affects
/// record indices and interleaving order.
fn parse_record_blocks(
    blocks: &[SchDocRawBlock],
    stream_name: &str,
) -> Result<Vec<(usize, RecordNode)>> {
    let mut records = Vec::new();
    for (index, block) in blocks.iter().enumerate() {
        if index == 0 {
            continue;
        }
        let block_offset = block.offset;

        if block.flags != 0 {
            if block.flags != 0x01 {
                return Err(AltiumError::Parse(format!(
                    "schdoc {} block at offset {} has unsupported binary flags {}",
                    stream_name, block_offset, block.flags
                )));
            }
            let record_data = block.payload.clone();
            if record_data.len() < 4 {
                return Err(AltiumError::Parse(format!(
                    "schdoc {} binary block too short at offset {} (len={})",
                    stream_name,
                    block_offset,
                    record_data.len()
                )));
            }
            let record_type = u32::from_le_bytes([
                record_data[0],
                record_data[1],
                record_data[2],
                record_data[3],
            ]) as u8;
            if !is_supported_sch_record_id(record_type) {
                return Err(AltiumError::Parse(format!(
                    "schdoc {} contains unimplemented record_id={} at offset {}",
                    stream_name, record_type, block_offset
                )));
            }
            let mut full_raw = Vec::with_capacity(4 + record_data.len());
            let header = ((block.flags as u32) << 24) | (record_data.len() as u32);
            full_raw.extend_from_slice(&header.to_le_bytes());
            full_raw.extend_from_slice(&record_data);
            let origin = RecordOrigin::Binary(crate::backing_store::BinaryOrigin::new(record_data));
            let mut node = RecordNode::new(record_type, origin);
            node.original_snapshot = full_raw;
            node.stream_name = Some(stream_name.to_string());
            records.push((index, node));
        } else {
            let record_data = block.payload.clone();
            let param_str = super::encoding::decode_win1252(&record_data);
            let params = ParameterCollection::from_string(&param_str);
            let record_id = params.get("RECORD").map(|v| v.as_int_or(0) as u8);

            if record_id.is_none() || record_id == Some(0) {
                return Err(AltiumError::Parse(format!(
                    "schdoc {} contains unimplemented non-record text block at offset {}",
                    stream_name, block_offset
                )));
            }
            let record_id = record_id.unwrap_or_default();
            if !is_supported_sch_record_id(record_id) {
                return Err(AltiumError::Parse(format!(
                    "schdoc {} contains unimplemented record_id={} at offset {}",
                    stream_name, record_id, block_offset
                )));
            }

            let origin = RecordOrigin::Param(ParamOrigin::new(&param_str));
            let mut node = RecordNode::new(record_id, origin);
            node.original_snapshot = record_data;
            node.stream_name = Some(stream_name.to_string());
            records.push((index, node));
        }
    }

    Ok(records)
}

/// Group parsed records by OWNERINDEX into the DocumentStore.
///
/// Component records (RECORD=1) form group parents. Other records are assigned
/// to the component whose group-order index matches their OWNERINDEX value.
/// Records with no valid owner become orphans.
fn group_by_owner_index(store: &mut DocumentStore, records: Vec<(usize, RecordNode)>) {
    let mut component_records: Vec<(usize, RecordNode)> = Vec::new();
    let mut child_records: Vec<(usize, RecordNode)> = Vec::new();

    for (flat_idx, node) in records {
        if node.key == 1 {
            component_records.push((flat_idx, node));
        } else {
            child_records.push((flat_idx, node));
        }
    }

    // Insert parent records and build the group list.
    // group_entries[i] = (GroupId, parent RecordId, Vec<(flat_idx, RecordId)>)
    let mut group_entries: Vec<(GroupId, RecordId, Vec<(usize, RecordId)>)> =
        Vec::with_capacity(component_records.len());

    for (flat_idx, comp_node) in component_records {
        let parent_id = store.insert_record(comp_node);
        let group_data = GroupData {
            parent: parent_id,
            children: Vec::new(),
            original_indices: Vec::new(),
            parent_original_index: Some(flat_idx),
            meta: GroupMeta::SchDocComponent,
        };
        let gid = store.insert_group(group_data);
        group_entries.push((gid, parent_id, Vec::new()));
    }

    // Assign children by OWNERINDEX.
    for (flat_idx, node) in child_records {
        let owner_index = match &node.origin {
            RecordOrigin::Param(p) => p
                .params
                .get("OWNERINDEX")
                .map(|v| v.as_int_or(-1))
                .unwrap_or(-1),
            _ => -1,
        };

        if owner_index >= 0 && (owner_index as usize) < group_entries.len() {
            let child_id = store.insert_record(node);
            group_entries[owner_index as usize]
                .2
                .push((flat_idx, child_id));
        } else {
            let child_id = store.insert_record(node);
            store.orphan_records.push(child_id);
            store.orphan_original_indices.push(flat_idx);
        }
    }

    // Populate children and original_indices on each group.
    for (gid, _parent_id, children) in group_entries {
        let group = store.group_mut(gid);
        for (flat_idx, child_id) in children {
            group.original_indices.push(flat_idx);
            group.children.push(child_id);
        }
    }
}

struct SchDocSerializedStreams {
    file_header: Vec<u8>,
    additional: Vec<u8>,
    storage: Vec<u8>,
}

fn default_schdoc_stream_for_record(record_id: u8) -> &'static str {
    if record_id == SchBlanketRecord::RECORD_ID {
        STREAM_ADDITIONAL
    } else {
        STREAM_FILE_HEADER
    }
}

fn effective_stream_name(node: &RecordNode) -> &str {
    node.stream_name
        .as_deref()
        .unwrap_or_else(|| default_schdoc_stream_for_record(node.key))
}

fn push_to_stream_timeline(
    stream_name: &str,
    idx: usize,
    order: usize,
    rid: RecordId,
    file_header_timeline: &mut Vec<(usize, usize, RecordId)>,
    additional_timeline: &mut Vec<(usize, usize, RecordId)>,
) {
    if stream_name.eq_ignore_ascii_case(STREAM_ADDITIONAL) {
        additional_timeline.push((idx, order, rid));
    } else {
        file_header_timeline.push((idx, order, rid));
    }
}

/// Flatten the document back into SchDoc streams for writing.
///
/// Records are emitted in per-stream flat index order so parent/child
/// interleaving is preserved within each stream.
fn flatten_to_streams(doc: &SchDoc) -> Result<SchDocSerializedStreams> {
    let mut file_header_data = Vec::new();
    let mut additional_data = Vec::new();
    let store = doc.store.borrow();
    let (file_header_meta, additional_meta, storage_meta) = match store.meta() {
        DocumentMeta::SchDoc {
            file_header_meta,
            additional_meta,
            storage_meta,
        } => (
            file_header_meta.clone(),
            additional_meta.clone(),
            storage_meta.clone(),
        ),
        _ => (
            SchDocFileHeaderStreamMeta::default(),
            None,
            SchDocStorageStreamMeta::default(),
        ),
    };

    // (original_index, insertion_order, record_id)
    let mut file_header_timeline: Vec<(usize, usize, RecordId)> = Vec::new();
    let mut additional_timeline: Vec<(usize, usize, RecordId)> = Vec::new();
    let mut insertion_order = 0usize;

    for &gid in store.group_ids() {
        let group = store.group(gid);
        let parent_id = group.parent_id();
        let parent_node = store.record(parent_id);
        let parent_idx = group.parent_original_index.unwrap_or(usize::MAX);
        let parent_stream = effective_stream_name(parent_node);
        push_to_stream_timeline(
            parent_stream,
            parent_idx,
            insertion_order,
            parent_id,
            &mut file_header_timeline,
            &mut additional_timeline,
        );
        insertion_order += 1;

        for (pos, &cid) in group.child_ids().iter().enumerate() {
            let idx = group
                .original_indices
                .get(pos)
                .copied()
                .unwrap_or(usize::MAX);
            let node = store.record(cid);
            let stream = effective_stream_name(node);
            push_to_stream_timeline(
                stream,
                idx,
                insertion_order,
                cid,
                &mut file_header_timeline,
                &mut additional_timeline,
            );
            insertion_order += 1;
        }
    }

    for (pos, &oid) in store.orphan_ids().iter().enumerate() {
        let idx = store
            .orphan_original_indices
            .get(pos)
            .copied()
            .unwrap_or(usize::MAX);
        let node = store.record(oid);
        let stream = effective_stream_name(node);
        push_to_stream_timeline(
            stream,
            idx,
            insertion_order,
            oid,
            &mut file_header_timeline,
            &mut additional_timeline,
        );
        insertion_order += 1;
    }

    file_header_timeline.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    additional_timeline.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

    for (_, _, rid) in file_header_timeline.iter().copied() {
        let node = store.record(rid);
        super::schlib::write_record_to_stream(&mut file_header_data, node)?;
    }
    for (_, _, rid) in additional_timeline.iter().copied() {
        let node = store.record(rid);
        super::schlib::write_record_to_stream(&mut additional_data, node)?;
    }

    let mut file_header_with_block =
        file_header_meta.serialize_header_block(file_header_timeline.len())?;
    file_header_with_block.extend_from_slice(&file_header_data);

    let additional_with_block = if additional_timeline.is_empty() && additional_meta.is_none() {
        Vec::new()
    } else {
        let meta = additional_meta.unwrap_or_default();
        let weight_override = if additional_timeline.is_empty() {
            None
        } else {
            Some(additional_timeline.len())
        };
        let mut stream = meta.serialize_header_block(weight_override)?;
        stream.extend_from_slice(&additional_data);
        stream
    };

    Ok(SchDocSerializedStreams {
        file_header: file_header_with_block,
        additional: additional_with_block,
        storage: storage_meta.to_stream_bytes()?,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backing_store::RecordOrigin;
    use std::io::{Cursor, Write};

    fn make_store_with_records(records: Vec<(usize, RecordNode)>) -> DocumentStore {
        let mut store = DocumentStore::new(DocumentMeta::SchDoc {
            file_header_meta: SchDocFileHeaderStreamMeta::default(),
            additional_meta: None,
            storage_meta: SchDocStorageStreamMeta::default(),
        });
        group_by_owner_index(&mut store, records);
        store
    }

    fn encode_test_block(flags: u8, payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let header = ((flags as u32) << 24) | (payload.len() as u32);
        out.extend_from_slice(&header.to_le_bytes());
        out.extend_from_slice(payload);
        out
    }

    fn make_schdoc_cfb_bytes(
        file_header: &[u8],
        additional: Option<&[u8]>,
        storage: &[u8],
        extra_stream: Option<(&str, &[u8])>,
    ) -> Vec<u8> {
        let mut cfb =
            cfb::CompoundFile::create_with_version(cfb::Version::V3, Cursor::new(Vec::new()))
                .unwrap();

        cfb.create_stream("/FileHeader")
            .unwrap()
            .write_all(file_header)
            .unwrap();
        if let Some(additional) = additional {
            cfb.create_stream("/Additional")
                .unwrap()
                .write_all(additional)
                .unwrap();
        }
        cfb.create_stream("/Storage")
            .unwrap()
            .write_all(storage)
            .unwrap();
        if let Some((name, bytes)) = extra_stream {
            cfb.create_stream(name).unwrap().write_all(bytes).unwrap();
        }

        cfb.flush().unwrap();
        cfb.into_inner().into_inner()
    }

    #[test]
    fn owner_index_grouping() {
        let records = vec![
            (
                0,
                RecordNode::new(
                    1,
                    RecordOrigin::Param(ParamOrigin::new("|RECORD=1|DESIGNATOR=U1|")),
                ),
            ),
            (
                1,
                RecordNode::new(
                    2,
                    RecordOrigin::Param(ParamOrigin::new("|RECORD=2|OWNERINDEX=0|NAME=VCC|")),
                ),
            ),
            (
                2,
                RecordNode::new(
                    2,
                    RecordOrigin::Param(ParamOrigin::new("|RECORD=2|OWNERINDEX=0|NAME=GND|")),
                ),
            ),
            (
                3,
                RecordNode::new(
                    1,
                    RecordOrigin::Param(ParamOrigin::new("|RECORD=1|DESIGNATOR=R1|")),
                ),
            ),
            (
                4,
                RecordNode::new(
                    2,
                    RecordOrigin::Param(ParamOrigin::new("|RECORD=2|OWNERINDEX=1|NAME=1|")),
                ),
            ),
        ];

        let store = make_store_with_records(records);

        assert_eq!(store.group_count(), 2);

        let group_ids: Vec<GroupId> = store.group_ids().to_vec();
        assert_eq!(store.group(group_ids[0]).child_ids().len(), 2);
        assert_eq!(store.group(group_ids[1]).child_ids().len(), 1);
    }

    #[test]
    fn orphan_records_collected() {
        let records = vec![
            (
                0,
                RecordNode::new(
                    1,
                    RecordOrigin::Param(ParamOrigin::new("|RECORD=1|DESIGNATOR=U1|")),
                ),
            ),
            (
                1,
                RecordNode::new(34, RecordOrigin::Param(ParamOrigin::new("|RECORD=34|"))),
            ),
        ];

        let store = make_store_with_records(records);

        assert_eq!(store.group_count(), 1);
        assert_eq!(store.orphan_ids().len(), 1);
        assert_eq!(store.record(store.orphan_ids()[0]).key, 34);
    }

    #[test]
    fn empty_stream_produces_empty_doc() {
        let records: Vec<(usize, RecordNode)> = Vec::new();
        let store = make_store_with_records(records);

        assert_eq!(store.group_count(), 0);
        assert!(store.orphan_ids().is_empty());
    }

    #[test]
    fn component_count_and_orphan_count() {
        let records = vec![
            (
                0,
                RecordNode::new(
                    1,
                    RecordOrigin::Param(ParamOrigin::new("|RECORD=1|DESIGNATOR=U1|")),
                ),
            ),
            (
                1,
                RecordNode::new(31, RecordOrigin::Param(ParamOrigin::new("|RECORD=31|"))),
            ),
        ];

        let store = std::rc::Rc::new(std::cell::RefCell::new(make_store_with_records(records)));
        let doc = SchDoc { store };

        assert_eq!(doc.component_count(), 1);
        assert_eq!(doc.orphan_count(), 1);
    }

    #[test]
    fn count_record_type_across_groups_and_orphans() {
        let records = vec![
            (
                0,
                RecordNode::new(
                    1,
                    RecordOrigin::Param(ParamOrigin::new("|RECORD=1|DESIGNATOR=U1|")),
                ),
            ),
            (
                1,
                RecordNode::new(
                    2,
                    RecordOrigin::Param(ParamOrigin::new("|RECORD=2|OWNERINDEX=0|NAME=VCC|")),
                ),
            ),
            (
                2,
                RecordNode::new(
                    2,
                    RecordOrigin::Param(ParamOrigin::new("|RECORD=2|OWNERINDEX=0|NAME=GND|")),
                ),
            ),
            (
                3,
                RecordNode::new(31, RecordOrigin::Param(ParamOrigin::new("|RECORD=31|"))),
            ),
        ];

        let store = std::rc::Rc::new(std::cell::RefCell::new(make_store_with_records(records)));
        let doc = SchDoc { store };

        assert_eq!(doc.count_record_type(1), 1);
        assert_eq!(doc.count_record_type(2), 2);
        assert_eq!(doc.count_record_type(31), 1);
    }

    #[test]
    fn schdoc_query_component() {
        use crate::traits::DocumentQuery;

        let records = vec![
            (
                0,
                RecordNode::new(
                    1,
                    RecordOrigin::Param(ParamOrigin::new("|RECORD=1|DESIGNATOR=U1|")),
                ),
            ),
            (
                1,
                RecordNode::new(
                    2,
                    RecordOrigin::Param(ParamOrigin::new(
                        "|RECORD=2|OWNERINDEX=0|Name=VCC|Designator=1|",
                    )),
                ),
            ),
            (
                2,
                RecordNode::new(
                    1,
                    RecordOrigin::Param(ParamOrigin::new("|RECORD=1|DESIGNATOR=R1|")),
                ),
            ),
        ];

        let store = std::rc::Rc::new(std::cell::RefCell::new(make_store_with_records(records)));
        let doc = SchDoc { store };

        let handle = DocumentQuery::<crate::handles::SchComponent>::query(&doc, "U1").unwrap();
        let comp = handle.read();
        assert_eq!(&*comp.designator().unwrap(), "U1");
    }

    #[test]
    fn schdoc_query_all_components() {
        use crate::traits::DocumentQuery;

        let records = vec![
            (
                0,
                RecordNode::new(
                    1,
                    RecordOrigin::Param(ParamOrigin::new("|RECORD=1|DESIGNATOR=U1|")),
                ),
            ),
            (
                1,
                RecordNode::new(
                    1,
                    RecordOrigin::Param(ParamOrigin::new("|RECORD=1|DESIGNATOR=R1|")),
                ),
            ),
        ];

        let store = std::rc::Rc::new(std::cell::RefCell::new(make_store_with_records(records)));
        let doc = SchDoc { store };

        let handles = DocumentQuery::<crate::handles::SchComponent>::query_all(&doc, "#1").unwrap();
        assert_eq!(handles.len(), 2);
    }

    #[test]
    fn schdoc_deep_query_children() {
        let records = vec![
            (
                0,
                RecordNode::new(
                    1,
                    RecordOrigin::Param(ParamOrigin::new("|RECORD=1|DESIGNATOR=U1|")),
                ),
            ),
            (
                1,
                RecordNode::new(
                    2,
                    RecordOrigin::Param(ParamOrigin::new(
                        "|RECORD=2|OWNERINDEX=0|Name=VCC|Designator=1|",
                    )),
                ),
            ),
            (
                2,
                RecordNode::new(
                    2,
                    RecordOrigin::Param(ParamOrigin::new(
                        "|RECORD=2|OWNERINDEX=0|Name=GND|Designator=2|",
                    )),
                ),
            ),
        ];

        let store = std::rc::Rc::new(std::cell::RefCell::new(make_store_with_records(records)));
        let doc = SchDoc { store };

        let handles = doc
            .query_all_children::<crate::handles::SchPin>("pin")
            .unwrap();
        assert_eq!(handles.len(), 2);
    }

    #[test]
    fn flatten_preserves_parent_child_orphan_interleaving() {
        let records = vec![
            (
                10,
                RecordNode::new(
                    1,
                    RecordOrigin::Param(ParamOrigin::new("|RECORD=1|DESIGNATOR=U1|")),
                ),
            ),
            (
                20,
                RecordNode::new(31, RecordOrigin::Param(ParamOrigin::new("|RECORD=31|"))),
            ),
            (
                30,
                RecordNode::new(
                    2,
                    RecordOrigin::Param(ParamOrigin::new(
                        "|RECORD=2|OWNERINDEX=0|NAME=VCC|DESIGNATOR=1|",
                    )),
                ),
            ),
        ];

        let store = std::rc::Rc::new(std::cell::RefCell::new(make_store_with_records(records)));
        let doc = SchDoc { store };

        let streams = flatten_to_streams(&doc).unwrap();
        let (_, blocks) = parse_file_header_meta_and_blocks(&streams.file_header).unwrap();
        let parsed = parse_record_blocks(&blocks, STREAM_FILE_HEADER).unwrap();
        let keys: Vec<u8> = parsed.into_iter().map(|(_, node)| node.key).collect();
        assert_eq!(keys, vec![1, 31, 2]);
    }

    #[test]
    fn open_rejects_unimplemented_root_streams() {
        let file_header = SchDocFileHeaderStreamMeta::default().serialize_header_block(0).unwrap();
        let storage = SchDocStorageStreamMeta::default().to_stream_bytes().unwrap();
        let bytes = make_schdoc_cfb_bytes(&file_header, None, &storage, Some(("/Unknown", b"x")));

        let err = SchDoc::open(Cursor::new(bytes)).err().unwrap();
        assert!(format!("{err}").contains("unimplemented stream"));
    }

    #[test]
    fn open_rejects_malformed_storage_entries() {
        let file_header = SchDocFileHeaderStreamMeta::default().serialize_header_block(0).unwrap();
        let mut storage = SchDocStorageStreamMeta::default().to_stream_bytes().unwrap();
        storage.extend_from_slice(&encode_test_block(0x00, &[0x00]));

        let bytes = make_schdoc_cfb_bytes(&file_header, None, &storage, None);
        let err = SchDoc::open(Cursor::new(bytes)).err().unwrap();
        assert!(format!("{err}").contains("Storage block"));
    }

    #[test]
    fn new_empty_build_component_and_orphan() {
        use crate::newtypes::Designator;
        use crate::records::SchWireRecord;
        use crate::templates;

        let doc = SchDoc::new_empty();
        let comp = doc.build_component(templates::sch_component_default, |builder| {
            builder.with_component(|c| {
                c.set_designator(Designator::from("U1"));
            });
            builder.add_pin(templates::sch_pin_default, |p| {
                p.set_designator(Designator::from("1"));
            });
        });

        assert_eq!(doc.component_count(), 1);
        assert_eq!(comp.children_len(), 1);

        let orphan = SchWireRecord::from_origin(templates::sch_wire_default());
        doc.add_orphan_record(orphan);
        assert_eq!(doc.orphan_count(), 1);

        let orphan_types: Vec<u8> = doc.orphan_records().into_iter().map(|(t, _)| t).collect();
        assert_eq!(orphan_types, vec![27]);
    }
}
