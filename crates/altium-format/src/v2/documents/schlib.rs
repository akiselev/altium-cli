//! SchLib document I/O using the v2 backing-store architecture.
//!
//! A SchLib file is a CFB compound file containing:
//! - `/FileHeader` stream: library metadata and component list
//! - `/SectionKeys` stream (optional): maps long component names to short CFB keys
//! - `/<ComponentKey>/Data` streams: per-component record data
//!
//! Each component's Data stream contains length-prefixed records as
//! pipe-delimited parameter strings. The first record is the component itself
//! (RECORD=1), followed by child records (pins, labels, etc.).

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{Cursor, Read, Seek, Write};
use std::rc::Rc;

use encoding_rs::WINDOWS_1252;

use crate::error::{AltiumError, Result};
use crate::v2::backing_store::{ParamOrigin, RecordNode, RecordOrigin};
use crate::v2::ids::GroupId;
use crate::v2::parameters::ParameterCollection;
use crate::v2::records::{SchComponentRecord, SchPinRecord};
use crate::v2::store::{DocRef, DocumentMeta, DocumentStore, GroupData, GroupMeta};
use crate::v2::traits::{HandleFamily, RecordType};

use super::section_keys::SectionKeyList;

// Stream name constants
const STREAM_FILE_HEADER: &str = "FileHeader";
const STREAM_SECTION_KEYS: &str = "SectionKeys";
const STREAM_DATA: &str = "Data";

// Size flag mask: low 24 bits = length, bit 24+ = binary mode flag
const SIZE_FLAG_MASK: u32 = 0x00FF_FFFF;

/// SchLib header info.
#[derive(Clone, Debug, Default)]
pub struct SchLibHeader {
    /// Header identification text (e.g. "Protel for Windows - Schematic Library Editor Binary File Version 5.0").
    pub header_text: String,
    /// Font weight.
    pub weight: i32,
    /// File format minor version.
    pub minor_version: i32,
    /// Unique ID for the library.
    pub unique_id: String,
    /// Raw bytes of the FileHeader stream (for identity write-back).
    pub raw: Option<Vec<u8>>,
    /// Parsed FileHeader parameters preserved for unknown-key round-tripping.
    pub raw_params: Option<ParameterCollection>,
}

impl SchLibHeader {
    /// Returns the unique ID.
    pub fn unique_id(&self) -> &str {
        &self.unique_id
    }

    /// Returns the header text.
    pub fn header_text(&self) -> &str {
        &self.header_text
    }

    /// Clears the raw bytes (forces re-serialization on save).
    pub fn clear_raw(&mut self) {
        self.raw = None;
    }
}

/// Component entry from the FileHeader's component list.
#[derive(Clone, Debug, Default)]
pub struct SchLibComponentEntry {
    /// Library reference name (the component's display name).
    pub lib_ref: String,
    /// Component description.
    pub description: String,
    /// Number of parts in the component.
    pub part_count: i32,
    /// Alias names for the component.
    pub aliases: Vec<String>,
}

impl SchLibComponentEntry {
    /// Library reference name.
    pub fn lib_ref(&self) -> &str {
        &self.lib_ref
    }

    /// Component description.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Number of parts.
    pub fn part_count(&self) -> i32 {
        self.part_count
    }

    /// Component aliases.
    pub fn aliases(&self) -> &[String] {
        &self.aliases
    }
}

/// A parsed SchLib library using the v2 DocumentStore architecture.
///
/// Preserves raw data for unmodified records to enable identity write-back.
pub struct SchLib {
    store: DocRef,
}

impl SchLib {
    /// Create a new empty SchLib document.
    pub fn new_empty() -> Self {
        let mut store = DocumentStore::new(DocumentMeta::SchLib {
            header_text: String::new(),
            weight: 0,
            minor_version: 0,
            unique_id: String::new(),
            raw_header: None,
            raw_header_params: None,
            section_keys: SectionKeyList::new(),
            raw_extra_streams: HashMap::new(),
        });
        store.set_semantic_context("dtid:schlib", "");
        Self {
            store: Rc::new(RefCell::new(store)),
        }
    }

    /// Returns a reference to the underlying document store.
    pub fn store(&self) -> &DocRef {
        &self.store
    }

    /// Replace library header metadata used when writing `/FileHeader`.
    ///
    /// Raw header passthrough fields are intentionally cleared so subsequent
    /// saves re-emit header text from modeled values.
    pub fn set_header(&self, header: &SchLibHeader) {
        let mut store = self.store.borrow_mut();
        let mut did_doc_key: Option<String> = None;
        if let DocumentMeta::SchLib {
            header_text,
            weight,
            minor_version,
            unique_id,
            raw_header,
            raw_header_params,
            ..
        } = &mut store.meta
        {
            *header_text = header.header_text.clone();
            *weight = header.weight;
            *minor_version = header.minor_version;
            *unique_id = header.unique_id.clone();
            *raw_header = None;
            *raw_header_params = header.raw_params.clone();
            did_doc_key = Some(unique_id.clone());
            store.mark_semantic_ids_dirty();
        }
        if let Some(doc_key) = did_doc_key {
            store.set_semantic_context("dtid:schlib", &doc_key);
        }
    }

    /// Returns the stable document-level semantic ID, if computed.
    pub fn document_id(&self) -> Option<crate::v2::semantic_ids::SemanticId> {
        let mut store = self.store.borrow_mut();
        store.ensure_semantic_ids();
        store.document_id().cloned()
    }

    /// Open a SchLib from a reader (CFB compound file).
    pub fn open<R: Read + Seek>(mut reader: R) -> Result<Self> {
        let mut raw_bytes = Vec::new();
        reader
            .read_to_end(&mut raw_bytes)
            .map_err(AltiumError::Io)?;
        let file_hash = crate::v2::semantic_ids::blake3_content_hash(&raw_bytes);

        let mut cfb = cfb::CompoundFile::open(Cursor::new(raw_bytes))
            .map_err(|e| AltiumError::Cfb(format!("Failed to open CFB: {}", e)))?;

        // 1. Read FileHeader
        let mut component_entries: Vec<SchLibComponentEntry> = Vec::new();
        let (header, parsed_header_params) = read_file_header(&mut cfb, &mut component_entries)?;
        let header_params = Some(parsed_header_params);

        // 2. Read SectionKeys
        let section_keys = read_section_keys(&mut cfb)?;

        // Collect all stream paths for capturing extra streams
        let all_stream_paths: Vec<String> = cfb
            .walk()
            .filter(|e| e.is_stream())
            .filter_map(|e| Some(e.path().to_str()?.to_string()))
            .collect();

        // Track which section keys are used by components
        let mut component_keys: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        // Build library-level extra_streams (determined before group insertion)
        let mut raw_extra_streams: HashMap<String, Vec<u8>> = HashMap::new();

        // Collect component_keys first pass
        for entry in &component_entries {
            let safe_name = sanitize_cfb_name(&entry.lib_ref);
            let section_key = section_keys.get_key(&safe_name).to_string();
            component_keys.insert(section_key);
        }

        // Capture library-level extra streams
        for stream_path in &all_stream_paths {
            let path_no_slash = stream_path.trim_start_matches('/');
            let top_level = path_no_slash.split('/').next().unwrap_or("");
            if top_level == STREAM_FILE_HEADER
                || top_level == STREAM_SECTION_KEYS
                || component_keys.contains(top_level)
            {
                continue;
            }
            if let Ok(mut stream) = cfb.open_stream(stream_path) {
                let mut data = Vec::new();
                if stream.read_to_end(&mut data).is_ok() {
                    raw_extra_streams.insert(path_no_slash.to_string(), data);
                }
            }
        }

        // Create DocumentStore with SchLib metadata
        let mut store = DocumentStore::new(DocumentMeta::SchLib {
            header_text: header.header_text.clone(),
            weight: header.weight,
            minor_version: header.minor_version,
            unique_id: header.unique_id.clone(),
            raw_header: header.raw.clone(),
            raw_header_params: header_params,
            section_keys: section_keys.clone(),
            raw_extra_streams,
        });

        // 3. Read Data stream for each component and insert into store
        for entry in &component_entries {
            let safe_name = sanitize_cfb_name(&entry.lib_ref);
            let section_key = section_keys.get_key(&safe_name).to_string();
            let data_path = format!("/{}/{}", section_key, STREAM_DATA);

            let (parent_node, children, original_indices) =
                if let Ok(mut stream) = cfb.open_stream(&data_path) {
                    let mut data = Vec::new();
                    stream.read_to_end(&mut data).map_err(AltiumError::Io)?;
                    parse_data_stream_to_group(&data)?
                } else {
                    let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=1|"));
                    (RecordNode::new(1, origin), Vec::new(), Vec::new())
                };

            // Capture extra streams in this component's storage
            let storage_prefix = format!("/{}/", section_key);
            let mut extra_streams: HashMap<String, Vec<u8>> = HashMap::new();
            for stream_path in &all_stream_paths {
                if let Some(rest) = stream_path.strip_prefix(&storage_prefix) {
                    if rest == STREAM_DATA {
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

            // Insert parent record
            let parent_id = store.insert_record(parent_node);

            // Insert child records
            let mut child_ids = Vec::with_capacity(children.len());
            for child in children {
                let id = store.insert_record(child);
                child_ids.push(id);
            }

            // Create GroupData with SchComponent metadata
            let group_data = GroupData {
                parent: parent_id,
                children: child_ids,
                original_indices,
                parent_original_index: None,
                extra_streams,
                meta: GroupMeta::SchComponent {
                    lib_ref: entry.lib_ref.clone(),
                    description: entry.description.clone(),
                    part_count: entry.part_count,
                    section_key,
                },
            };

            store.insert_group(group_data);
        }

        let doc_key = if header.unique_id.trim().is_empty() {
            file_hash
        } else {
            header.unique_id.clone()
        };
        crate::v2::semantic_ids::compute_all_ids(&mut store, "dtid:schlib", &doc_key);

        Ok(SchLib {
            store: Rc::new(RefCell::new(store)),
        })
    }

    /// Open a SchLib from a file path.
    pub fn open_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let file = std::fs::File::open(path).map_err(AltiumError::Io)?;
        Self::open(file)
    }

    /// Save the SchLib to a writer (creates a new CFB compound file).
    pub fn save<W: Read + Write + Seek>(&self, writer: W) -> Result<()> {
        let mut store = self.store.borrow_mut();
        store.ensure_semantic_ids();

        // Extract SchLib metadata
        let (
            header_text,
            weight,
            minor_version,
            unique_id,
            raw_header,
            raw_header_params,
            _stored_section_keys,
            raw_extra_streams,
        ) = match &store.meta {
            DocumentMeta::SchLib {
                header_text,
                weight,
                minor_version,
                unique_id,
                raw_header,
                raw_header_params,
                section_keys,
                raw_extra_streams,
            } => (
                header_text.clone(),
                *weight,
                *minor_version,
                unique_id.clone(),
                raw_header.clone(),
                raw_header_params.clone(),
                section_keys.clone(),
                raw_extra_streams.clone(),
            ),
            _ => return Err(AltiumError::Cfb("Expected SchLib metadata".to_string())),
        };

        let mut cfb = cfb::CompoundFile::create(writer)
            .map_err(|e| AltiumError::Cfb(format!("Failed to create CFB: {}", e)))?;

        // Collect component entries from current store state.
        let mut component_entries: Vec<SchLibComponentEntry> = Vec::new();
        for &group_id in store.group_ids() {
            let group = store.group(group_id);
            match &group.meta {
                GroupMeta::SchComponent {
                    lib_ref,
                    description,
                    ..
                } => {
                    let parent_node = store.record(group.parent_id());
                    let comp_rec = SchComponentRecord::from_origin(parent_node.origin.clone());
                    component_entries.push(SchLibComponentEntry {
                        lib_ref: lib_ref.clone(),
                        description: description.clone(),
                        part_count: comp_rec.part_count() as i32,
                        aliases: parse_alias_list(&comp_rec.alias_list().to_string()),
                    });
                }
                _ => {}
            }
        }

        // 1. Build section keys
        let mut section_keys = SectionKeyList::new();
        for entry in &component_entries {
            let safe = sanitize_cfb_name(&entry.lib_ref);
            section_keys.add_key(&safe, 30);
            for alias in &entry.aliases {
                let safe_alias = sanitize_cfb_name(alias);
                section_keys.add_key(&safe_alias, 30);
            }
        }

        // 2. Write FileHeader — always rebuild from current store data so
        // that mutations (e.g. renamed components) are reflected while unknown
        // keys from the original header are preserved.
        {
            let _ = raw_header; // deliberately unused; always rebuild
            let header = SchLibHeader {
                header_text,
                weight,
                minor_version,
                unique_id,
                raw: None,
                raw_params: raw_header_params.clone(),
            };
            write_file_header(
                &mut cfb,
                &header,
                &component_entries,
                header.raw_params.as_ref(),
            )?;
        }

        // 3. Write SectionKeys
        write_section_keys(&mut cfb, &section_keys)?;

        // 4. Write Data stream for each component group
        for (i, &group_id) in store.group_ids().iter().enumerate() {
            let group = store.group(group_id);
            let lib_ref = match &group.meta {
                GroupMeta::SchComponent { lib_ref, .. } => lib_ref.clone(),
                _ => continue,
            };

            let safe_name = sanitize_cfb_name(&lib_ref);
            let section_key = section_keys.get_key(&safe_name).to_string();

            let storage_path = format!("/{}", section_key);
            cfb.create_storage(&storage_path)
                .map_err(|e| AltiumError::Cfb(format!("Failed to create storage: {}", e)))?;

            // Build data stream from store records
            let parent_node = store.record(group.parent_id());
            let mut data_bytes = Vec::new();
            write_record_to_schlib_stream(&mut data_bytes, parent_node)?;
            for &child_id in group.child_ids() {
                let child_node = store.record(child_id);
                write_record_to_schlib_stream(&mut data_bytes, child_node)?;
            }

            let data_path = format!("/{}/{}", section_key, STREAM_DATA);
            let mut stream = cfb
                .create_stream(&data_path)
                .map_err(|e| AltiumError::Cfb(format!("Failed to create Data stream: {}", e)))?;
            stream.write_all(&data_bytes).map_err(AltiumError::Io)?;

            // Write per-component extra streams
            {
                let mut created_storages: std::collections::HashSet<String> =
                    std::collections::HashSet::new();
                created_storages.insert(storage_path.clone());
                for (rel_path, extra_data) in &group.extra_streams {
                    let full_path = format!("/{}/{}", section_key, rel_path);
                    super::pcblib::ensure_parent_storages(
                        &mut cfb,
                        &full_path,
                        &mut created_storages,
                    )?;
                    let mut stream = cfb.create_stream(&full_path).map_err(|e| {
                        AltiumError::Cfb(format!(
                            "Failed to create extra stream {}: {}",
                            full_path, e
                        ))
                    })?;
                    stream.write_all(extra_data).map_err(AltiumError::Io)?;
                }
            }

            // Emit alias redirection streams when they are not already present in
            // preserved raw extra streams.
            if let Some(entry) = component_entries.get(i) {
                let mut created_storages: std::collections::HashSet<String> =
                    std::collections::HashSet::new();
                let mut emitted_redirections: std::collections::HashSet<String> =
                    std::collections::HashSet::new();
                for alias in &entry.aliases {
                    if alias.eq_ignore_ascii_case(&lib_ref) {
                        continue;
                    }
                    let alias_safe = sanitize_cfb_name(alias);
                    let alias_key = section_keys.get_key(&alias_safe).to_string();
                    let rel = format!("{}/Redirection", alias_key);
                    let rel_key = rel.to_ascii_lowercase();
                    if !emitted_redirections.insert(rel_key) {
                        continue;
                    }
                    if has_library_extra_stream(&raw_extra_streams, &rel) {
                        continue;
                    }
                    let full_path = format!("/{}", rel);
                    super::pcblib::ensure_parent_storages(
                        &mut cfb,
                        &full_path,
                        &mut created_storages,
                    )?;
                    let mut stream = cfb.create_stream(&full_path).map_err(|e| {
                        AltiumError::Cfb(format!(
                            "Failed to create redirection stream {}: {}",
                            full_path, e
                        ))
                    })?;
                    let data = build_section_redirection_stream_bytes(&lib_ref);
                    stream.write_all(&data).map_err(AltiumError::Io)?;
                }
            }
        }

        // 5. Write /Storage if not already preserved as raw extra data.
        if !raw_extra_streams
            .keys()
            .any(|k| k.eq_ignore_ascii_case("Storage"))
        {
            let mut stream = cfb
                .create_stream("/Storage")
                .map_err(|e| AltiumError::Cfb(format!("Failed to create /Storage: {}", e)))?;
            stream
                .write_all(&build_icon_storage_stream_bytes())
                .map_err(AltiumError::Io)?;
        }

        // 6. Write library-level extra streams
        {
            let mut created_storages: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            let mut sorted_extras: Vec<_> = raw_extra_streams.iter().collect();
            sorted_extras.sort_by_key(|(k, _)| (*k).clone());
            for (rel_path, data) in &sorted_extras {
                let full_path = format!("/{}", rel_path);
                super::pcblib::ensure_parent_storages(&mut cfb, &full_path, &mut created_storages)?;
                let mut stream = cfb.create_stream(&full_path).map_err(|e| {
                    AltiumError::Cfb(format!(
                        "Failed to create extra stream {}: {}",
                        full_path, e
                    ))
                })?;
                stream.write_all(data).map_err(AltiumError::Io)?;
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

    /// Returns the number of components in the library.
    pub fn component_count(&self) -> usize {
        self.store.borrow().group_count()
    }

    /// Returns the library reference names of all components.
    pub fn component_names(&self) -> Vec<String> {
        let store = self.store.borrow();
        store
            .group_ids()
            .iter()
            .filter_map(|&gid| {
                let group = store.group(gid);
                match &group.meta {
                    GroupMeta::SchComponent { lib_ref, .. } => Some(lib_ref.clone()),
                    _ => None,
                }
            })
            .collect()
    }

    /// Returns component entries derived from store group metadata.
    pub fn entries(&self) -> Vec<SchLibComponentEntry> {
        let store = self.store.borrow();
        store
            .group_ids()
            .iter()
            .filter_map(|&gid| {
                let group = store.group(gid);
                match &group.meta {
                    GroupMeta::SchComponent {
                        lib_ref,
                        description,
                        part_count,
                        ..
                    } => {
                        let parent_node = store.record(group.parent_id());
                        let comp_rec = SchComponentRecord::from_origin(parent_node.origin.clone());
                        Some(SchLibComponentEntry {
                            lib_ref: lib_ref.clone(),
                            description: description.clone(),
                            part_count: *part_count,
                            aliases: parse_alias_list(&comp_rec.alias_list().to_string()),
                        })
                    }
                    _ => None,
                }
            })
            .collect()
    }

    /// Returns the library header derived from store metadata.
    pub fn header(&self) -> SchLibHeader {
        let store = self.store.borrow();
        match &store.meta {
            DocumentMeta::SchLib {
                header_text,
                weight,
                minor_version,
                unique_id,
                raw_header,
                raw_header_params,
                ..
            } => SchLibHeader {
                header_text: header_text.clone(),
                weight: *weight,
                minor_version: *minor_version,
                unique_id: unique_id.clone(),
                raw: raw_header.clone(),
                raw_params: raw_header_params.clone(),
            },
            _ => SchLibHeader::default(),
        }
    }

    /// Find a component by name (case-insensitive), returns its GroupId.
    pub fn find_component(&self, name: &str) -> Option<GroupId> {
        let name_lower = name.to_lowercase();
        let store = self.store.borrow();
        store.group_ids().iter().find_map(|&gid| {
            let group = store.group(gid);
            match &group.meta {
                GroupMeta::SchComponent { lib_ref, .. } if lib_ref.to_lowercase() == name_lower => {
                    Some(gid)
                }
                _ => None,
            }
        })
    }

    /// Build and add a new component using the builder pattern.
    ///
    /// # Example
    ///
    /// ```ignore
    /// lib.build_component(templates::sch_component_default, |builder| {
    ///     builder.with_component(|comp| {
    ///         comp.set_lib_reference(LibReference::from("R_NEW"));
    ///     });
    ///     builder.add_pin(templates::sch_pin_default, |pin| {
    ///         pin.set_designator(Designator::from("1"));
    ///     });
    /// });
    /// ```
    pub fn build_component(
        &self,
        template: fn() -> RecordOrigin,
        build: impl FnOnce(&mut crate::v2::builders::ComponentBuilder),
    ) -> crate::v2::handles::SchComponentHandle {
        let mut builder = crate::v2::builders::ComponentBuilder::new(template);
        build(&mut builder);

        let (component, children) = builder.build();

        // Extract lib_ref and description from the built component record
        let comp_record =
            crate::v2::records::SchComponentRecord::from_origin(component.origin.clone());
        let lib_ref = comp_record.lib_reference().to_string();
        let description = comp_record.component_description().to_string();
        let part_count = comp_record.part_count() as i32;

        let mut store = self.store.borrow_mut();

        let parent_id = store.insert_record(component);
        let mut child_ids = Vec::with_capacity(children.len());
        for child in children {
            let id = store.insert_record(child);
            child_ids.push(id);
        }

        let group_data = GroupData {
            parent: parent_id,
            children: child_ids,
            original_indices: Vec::new(),
            parent_original_index: None,
            extra_streams: HashMap::new(),
            meta: GroupMeta::SchComponent {
                lib_ref,
                description,
                part_count,
                section_key: String::new(),
            },
        };

        let group_id = store.insert_group(group_data);
        store.mark_semantic_ids_dirty();
        crate::v2::handles::SchComponentHandle::new(self.store.clone(), group_id)
    }
}

// ---------------------------------------------------------------------------
// DocumentQuery<SchComponent> for SchLib
// ---------------------------------------------------------------------------

impl crate::v2::traits::DocumentQuery<crate::v2::handles::SchComponent> for SchLib {
    fn query(&self, q: &str) -> crate::error::Result<crate::v2::handles::SchComponentHandle> {
        use crate::v2::query::eval::evaluate;
        let parsed = crate::v2::query::parse(q)?;

        let store = self.store.borrow();
        let mut matches: Vec<GroupId> = Vec::new();

        for &group_id in store.group_ids() {
            let group = store.group(group_id);
            let node = store.record(group.parent_id());
            let all = std::slice::from_ref(node);
            if !evaluate(&parsed, all).is_empty() {
                matches.push(group_id);
            }
        }

        match matches.len() {
            0 => Err(crate::error::AltiumError::NoMatch(q.to_string())),
            1 => Ok(crate::v2::handles::SchComponentHandle::new(
                self.store.clone(),
                matches[0],
            )),
            n => Err(crate::error::AltiumError::AmbiguousMatch(n, q.to_string())),
        }
    }

    fn query_all(
        &self,
        q: &str,
    ) -> crate::error::Result<Vec<crate::v2::handles::SchComponentHandle>> {
        use crate::v2::query::eval::evaluate;
        let parsed = crate::v2::query::parse(q)?;

        let store = self.store.borrow();
        let mut handles = Vec::new();

        for &group_id in store.group_ids() {
            let group = store.group(group_id);
            let node = store.record(group.parent_id());
            let all = std::slice::from_ref(node);
            if !evaluate(&parsed, all).is_empty() {
                handles.push(crate::v2::handles::SchComponentHandle::new(
                    self.store.clone(),
                    group_id,
                ));
            }
        }

        Ok(handles)
    }
}

// ---------------------------------------------------------------------------
// Deep child queries for SchLib
// ---------------------------------------------------------------------------

impl SchLib {
    /// Query a single child record of type `T` across all component groups.
    pub fn query_child<T: HandleFamily>(&self, q: &str) -> crate::error::Result<T::Handle> {
        use crate::v2::query::eval::evaluate;
        let parsed = crate::v2::query::parse(q)?;

        let store = self.store.borrow();
        let mut matches = Vec::new();

        for &group_id in store.group_ids() {
            let group = store.group(group_id);
            for &child_id in group.child_ids() {
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
            1 => T::try_make_handle(self.store.clone(), matches[0]),
            n => Err(crate::error::AltiumError::AmbiguousMatch(n, q.to_string())),
        }
    }

    /// Query all child records of type `T` across all component groups.
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
            for &child_id in group.child_ids() {
                let node = store.record(child_id);
                if node.key == T::record_id() && node.origin.is_binary() == T::is_binary() {
                    let all = std::slice::from_ref(node);
                    if !evaluate(&parsed, all).is_empty() {
                        handles.push(T::try_make_handle(self.store.clone(), child_id)?);
                    }
                }
            }
        }

        Ok(handles)
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Replace characters that are invalid in CFB storage names.
fn sanitize_cfb_name(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '!' => '_',
            _ => c,
        })
        .collect()
}

fn parse_alias_list(alias_list: &str) -> Vec<String> {
    alias_list
        .split(',')
        .map(|s| s.trim().trim_matches('"'))
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// Decode raw bytes as Windows-1252 into a Rust String.
///
/// Altium files use Windows-1252 encoding for text records. Using this
/// instead of `String::from_utf8_lossy` preserves all byte values as
/// proper Unicode characters (e.g. `\xb5` → µ) instead of replacing
/// them with U+FFFD.
pub(super) fn decode_win1252(bytes: &[u8]) -> String {
    let (text, _, _) = WINDOWS_1252.decode(bytes);
    text.into_owned()
}

/// Encode a Rust String back to Windows-1252 bytes.
///
/// This is the inverse of `decode_win1252` — characters that originated
/// from Windows-1252 bytes are mapped back to their original single-byte
/// values, enabling byte-perfect round-tripping.
pub(super) fn encode_win1252(s: &str) -> Vec<u8> {
    let (bytes, _, _) = WINDOWS_1252.encode(s);
    bytes.into_owned()
}

fn parse_first_param_block(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 4 {
        return None;
    }
    let raw_len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if (raw_len & !SIZE_FLAG_MASK) != 0 {
        return None;
    }
    let len = (raw_len & SIZE_FLAG_MASK) as usize;
    if len == 0 || 4 + len > data.len() {
        return None;
    }
    Some(data[4..4 + len].to_vec())
}

fn encode_single_param_block(params: &ParameterCollection) -> Vec<u8> {
    let mut payload = encode_win1252(&params.to_param_string());
    payload.push(0);
    let mut out = Vec::with_capacity(payload.len() + 4);
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(&payload);
    out
}

fn build_section_redirection_stream_bytes(section_name: &str) -> Vec<u8> {
    let mut params = ParameterCollection::new();
    params.add("SECTIONNAME", section_name);
    encode_single_param_block(&params)
}

fn build_icon_storage_stream_bytes() -> Vec<u8> {
    let mut params = ParameterCollection::new();
    params.add("HEADER", "Icon storage");
    encode_single_param_block(&params)
}

fn has_library_extra_stream(raw_extra_streams: &HashMap<String, Vec<u8>>, rel_path: &str) -> bool {
    raw_extra_streams
        .keys()
        .any(|k| k.eq_ignore_ascii_case(rel_path))
}

/// Read and parse the FileHeader stream.
fn read_file_header<F: Read + Seek>(
    cfb: &mut cfb::CompoundFile<F>,
    entries: &mut Vec<SchLibComponentEntry>,
) -> Result<(SchLibHeader, ParameterCollection)> {
    let path = format!("/{}", STREAM_FILE_HEADER);
    let mut stream = cfb
        .open_stream(&path)
        .map_err(|e| AltiumError::Cfb(format!("No FileHeader: {}", e)))?;
    let mut data = Vec::new();
    stream.read_to_end(&mut data).map_err(AltiumError::Io)?;

    let payload = parse_first_param_block(&data).unwrap_or_else(|| data.clone());
    let text = decode_win1252(&payload);
    let params = ParameterCollection::from_string(&text);

    let header = SchLibHeader {
        header_text: params
            .get("HEADER")
            .map(|v| v.as_str().to_string())
            .unwrap_or_default(),
        weight: params.get("Weight").map(|v| v.as_int_or(0)).unwrap_or(0),
        minor_version: params
            .get("MinorVersion")
            .map(|v| v.as_int_or(0))
            .unwrap_or(0),
        unique_id: params
            .get("UniqueID")
            .map(|v| v.as_str().to_string())
            .unwrap_or_default(),
        raw: Some(data),
        raw_params: Some(params.clone()),
    };

    let comp_count = params.get("CompCount").map(|v| v.as_int_or(0)).unwrap_or(0);
    for i in 0..comp_count {
        let lib_ref = params
            .get(&format!("LibRef{}", i))
            .map(|v| v.as_str().to_string())
            .unwrap_or_default();
        let description = params
            .get(&format!("CompDescr{}", i))
            .map(|v| v.as_str().to_string())
            .unwrap_or_default();
        let part_count = params
            .get(&format!("PartCount{}", i))
            .map(|v| v.as_int_or(1))
            .unwrap_or(1);
        let alias_count = params
            .get(&format!("AliasCount{}", i))
            .map(|v| v.as_int_or(0))
            .unwrap_or(0)
            .max(0);
        let mut aliases = Vec::with_capacity(alias_count as usize);
        for j in 0..alias_count {
            if let Some(v) = params.get(&format!("Comp{}Alias{}", i, j)) {
                let alias = v.as_str().to_string();
                if !alias.is_empty() {
                    aliases.push(alias);
                }
            }
        }

        entries.push(SchLibComponentEntry {
            lib_ref,
            description,
            part_count,
            aliases,
        });
    }

    Ok((header, params))
}

/// Read the SectionKeys stream if present.
fn read_section_keys<F: Read + Seek>(cfb: &mut cfb::CompoundFile<F>) -> Result<SectionKeyList> {
    let mut keys = SectionKeyList::new();
    if let Ok(mut stream) = cfb.open_stream(format!("/{}", STREAM_SECTION_KEYS)) {
        let mut data = Vec::new();
        stream.read_to_end(&mut data).map_err(AltiumError::Io)?;
        let payload = parse_first_param_block(&data).unwrap_or(data);
        let text = decode_win1252(&payload);
        let params = ParameterCollection::from_string(&text);
        let count = params.get("KeyCount").map(|v| v.as_int_or(0)).unwrap_or(0);
        for i in 0..count {
            if let (Some(name_val), Some(key_val)) = (
                params
                    .get(&format!("LibRef{}", i))
                    .or_else(|| params.get(&format!("Key{}", i))),
                params.get(&format!("SectionKey{}", i)),
            ) {
                let name = name_val.as_str().to_string();
                let key = key_val.as_str().to_string();
                keys.insert_mapping(&name, &key);
            }
        }
    }
    Ok(keys)
}

/// Write the FileHeader stream from structured data.
fn write_file_header<F: Read + Write + Seek>(
    cfb: &mut cfb::CompoundFile<F>,
    header: &SchLibHeader,
    entries: &[SchLibComponentEntry],
    base_params: Option<&ParameterCollection>,
) -> Result<()> {
    let mut params = base_params
        .cloned()
        .unwrap_or_else(ParameterCollection::new);
    let old_count = params
        .get("CompCount")
        .map(|v| v.as_int_or(0).max(0) as usize)
        .unwrap_or(0);

    // Track which CompDescrN keys existed so we can avoid introducing empty
    // description keys for untouched legacy files.
    let mut had_compdescr: std::collections::HashSet<usize> = std::collections::HashSet::new();
    let mut to_remove = Vec::new();
    for (key, _) in params.iter() {
        if let Some(idx) = key
            .strip_prefix("LIBREF")
            .and_then(|s| s.parse::<usize>().ok())
        {
            let _ = idx;
            to_remove.push(key.to_string());
            continue;
        }
        if let Some(idx) = key
            .strip_prefix("COMPDESCR")
            .and_then(|s| s.parse::<usize>().ok())
        {
            had_compdescr.insert(idx);
            to_remove.push(key.to_string());
            continue;
        }
        if key
            .strip_prefix("PARTCOUNT")
            .and_then(|s| s.parse::<usize>().ok())
            .is_some()
        {
            to_remove.push(key.to_string());
            continue;
        }
        if key
            .strip_prefix("ALIASCOUNT")
            .and_then(|s| s.parse::<usize>().ok())
            .is_some()
        {
            to_remove.push(key.to_string());
            continue;
        }
        if let Some(rest) = key.strip_prefix("COMP") {
            let digit_count = rest.bytes().take_while(|b| b.is_ascii_digit()).count();
            if digit_count > 0 && rest[digit_count..].starts_with("ALIAS") {
                to_remove.push(key.to_string());
                continue;
            }
        }
    }
    for key in to_remove {
        params.remove(&key);
    }

    // Patch known modeled keys.
    params.add("HEADER", &header.header_text);
    params.add("WEIGHT", &header.weight.to_string());
    params.add("MINORVERSION", &header.minor_version.to_string());
    if !header.unique_id.is_empty() || params.contains("UNIQUEID") {
        params.add("UNIQUEID", &header.unique_id);
    } else {
        params.remove("UNIQUEID");
    }
    params.add("COMPCOUNT", &entries.len().to_string());

    for (i, entry) in entries.iter().enumerate() {
        params.add(&format!("LIBREF{}", i), &entry.lib_ref);
        if !entry.description.is_empty() || had_compdescr.contains(&i) || i >= old_count {
            params.add(&format!("COMPDESCR{}", i), &entry.description);
        }
        params.add(&format!("PARTCOUNT{}", i), &entry.part_count.to_string());
        if !entry.aliases.is_empty() {
            params.add(
                &format!("ALIASCOUNT{}", i),
                &entry.aliases.len().to_string(),
            );
            for (j, alias) in entry.aliases.iter().enumerate() {
                params.add(&format!("COMP{}ALIAS{}", i, j), alias);
            }
        }
    }

    let data = encode_single_param_block(&params);
    let path = format!("/{}", STREAM_FILE_HEADER);
    let mut stream = cfb
        .create_stream(&path)
        .map_err(|e| AltiumError::Cfb(format!("Failed to create FileHeader: {}", e)))?;
    stream.write_all(&data).map_err(AltiumError::Io)?;
    Ok(())
}

/// Write the SectionKeys stream.
fn write_section_keys<F: Read + Write + Seek>(
    cfb: &mut cfb::CompoundFile<F>,
    keys: &SectionKeyList,
) -> Result<()> {
    if keys.is_empty() {
        return Ok(());
    }
    let mut params = ParameterCollection::new();
    params.add("KeyCount", &keys.len().to_string());
    for (i, (name, key)) in keys.iter().enumerate() {
        params.add(&format!("LibRef{}", i), name);
        params.add(&format!("SectionKey{}", i), key);
    }
    let data = encode_single_param_block(&params);
    let path = format!("/{}", STREAM_SECTION_KEYS);
    let mut stream = cfb
        .create_stream(&path)
        .map_err(|e| AltiumError::Cfb(format!("Failed to create SectionKeys: {}", e)))?;
    stream.write_all(&data).map_err(AltiumError::Io)?;
    Ok(())
}

/// Parse a data stream into `(parent_node, children, original_indices)`.
///
/// The first record is the component (RECORD=1); remaining records are children.
fn parse_data_stream_to_group(data: &[u8]) -> Result<(RecordNode, Vec<RecordNode>, Vec<usize>)> {
    let records = parse_data_stream(data)?;

    if records.is_empty() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=1|"));
        return Ok((RecordNode::new(1, origin), Vec::new(), Vec::new()));
    }

    let mut iter = records.into_iter();
    let parent = iter.next().unwrap();
    let children: Vec<RecordNode> = iter.collect();
    let original_indices: Vec<usize> = (1..=children.len()).collect();

    Ok((parent, children, original_indices))
}

/// Parse a data stream into individual RecordNodes.
///
/// Each record is stored as a 4-byte little-endian length prefix followed by
/// the record data. The high byte of the length indicates binary vs text mode.
fn parse_data_stream(data: &[u8]) -> Result<Vec<RecordNode>> {
    let mut records = Vec::new();
    let mut cursor = Cursor::new(data);
    let total_len = data.len() as u64;

    while cursor.position() < total_len {
        let mut len_buf = [0u8; 4];
        if Read::read_exact(&mut cursor, &mut len_buf).is_err() {
            break;
        }
        let size_raw = u32::from_le_bytes(len_buf);
        let is_binary = (size_raw & !SIZE_FLAG_MASK) != 0;
        let record_len = (size_raw & SIZE_FLAG_MASK) as usize;

        if record_len == 0 {
            continue;
        }
        if cursor.position() as usize + record_len > data.len() {
            break;
        }

        let mut record_data = vec![0u8; record_len];
        if Read::read_exact(&mut cursor, &mut record_data).is_err() {
            break;
        }

        if is_binary {
            let record_type = if record_data.len() >= 4 {
                u32::from_le_bytes([
                    record_data[0],
                    record_data[1],
                    record_data[2],
                    record_data[3],
                ]) as u8
            } else {
                0
            };
            let mut full_raw = Vec::with_capacity(4 + record_len);
            full_raw.extend_from_slice(&len_buf);
            full_raw.extend_from_slice(&record_data);
            let origin =
                RecordOrigin::Binary(crate::v2::backing_store::BinaryOrigin::new(record_data));
            let mut node = RecordNode::new(record_type, origin);
            node.original_snapshot = full_raw;
            records.push(node);
        } else {
            let param_str = decode_win1252(&record_data);
            let params = ParameterCollection::from_string(&param_str);
            let record_id = params
                .get("RECORD")
                .map(|v| v.as_int_or(0) as u8)
                .unwrap_or(0);

            // Skip header markers (RECORD=0)
            if record_id == 0 {
                continue;
            }

            let origin = RecordOrigin::Param(ParamOrigin::new(&param_str));
            let mut node = RecordNode::new(record_id, origin);
            node.original_snapshot = record_data;
            records.push(node);
        }
    }

    Ok(records)
}

/// Write a single RecordNode to a data stream.
///
/// This is `pub(super)` so that sibling modules (e.g. schdoc) can reuse it.
pub(super) fn write_record_to_stream(output: &mut Vec<u8>, node: &RecordNode) -> Result<()> {
    if node.is_dirty() {
        // Re-serialize from origin
        match &node.origin {
            RecordOrigin::Param(p) => {
                let text = p.params.to_param_string();
                let bytes = encode_win1252(&text);
                let len = bytes.len() as u32;
                output.extend_from_slice(&len.to_le_bytes());
                output.extend_from_slice(&bytes);
            }
            RecordOrigin::Binary(b) => {
                let len = (b.raw_block.len() as u32) | 0x0100_0000; // set binary flag
                output.extend_from_slice(&len.to_le_bytes());
                output.extend_from_slice(&b.raw_block);
            }
        }
    } else {
        // Write original snapshot bytes
        match &node.origin {
            RecordOrigin::Param(_) => {
                let len = node.original_snapshot.len() as u32;
                output.extend_from_slice(&len.to_le_bytes());
                output.extend_from_slice(&node.original_snapshot);
            }
            RecordOrigin::Binary(_) => {
                // Binary snapshots include the length header
                output.extend_from_slice(&node.original_snapshot);
            }
        }
    }
    Ok(())
}

fn write_record_to_schlib_stream(output: &mut Vec<u8>, node: &RecordNode) -> Result<()> {
    if node.key == SchPinRecord::RECORD_ID {
        if let Some(param) = node.origin.as_param() {
            let pin = SchPinRecord::from_origin(RecordOrigin::Param(param.clone()));
            let raw = pin.to_legacy_binary_record_data();
            let len = (raw.len() as u32) | 0x0100_0000;
            output.extend_from_slice(&len.to_le_bytes());
            output.extend_from_slice(&raw);
            return Ok(());
        }
    }
    write_record_to_stream(output, node)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn make_lib_with_components(names: &[&str]) -> SchLib {
        let store = DocumentStore::new(DocumentMeta::SchLib {
            header_text: "Test".to_string(),
            weight: 0,
            minor_version: 0,
            unique_id: String::new(),
            raw_header: None,
            raw_header_params: None,
            section_keys: SectionKeyList::new(),
            raw_extra_streams: HashMap::new(),
        });
        let lib = SchLib {
            store: Rc::new(RefCell::new(store)),
        };

        for name in names {
            let param_str = format!("|RECORD=1|DESIGNATOR={}|LIBREFERENCE={}|", name, name);
            let origin = RecordOrigin::Param(ParamOrigin::new(&param_str));
            let parent = RecordNode::new(1, origin);

            let mut s = lib.store.borrow_mut();
            let parent_id = s.insert_record(parent);
            s.insert_group(GroupData {
                parent: parent_id,
                children: Vec::new(),
                original_indices: Vec::new(),
                parent_original_index: None,
                extra_streams: HashMap::new(),
                meta: GroupMeta::SchComponent {
                    lib_ref: name.to_string(),
                    description: String::new(),
                    part_count: 1,
                    section_key: String::new(),
                },
            });
        }

        lib
    }

    #[test]
    fn data_stream_roundtrip() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=1|LIBREFERENCE=LM358|PARTCOUNT=2|",
        ));
        let parent = RecordNode::new(1, origin);
        let pin_origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=2|OWNERINDEX=0|NAME=VCC|"));
        let pin = RecordNode::new(2, pin_origin);

        let mut data = Vec::new();
        write_record_to_stream(&mut data, &parent).unwrap();
        write_record_to_stream(&mut data, &pin).unwrap();

        let (parsed_parent, children, _) = parse_data_stream_to_group(&data).unwrap();

        assert_eq!(parsed_parent.key, 1);
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].key, 2);
    }

    #[test]
    fn cfb_roundtrip() {
        let store = DocumentStore::new(DocumentMeta::SchLib {
            header_text: "Test".to_string(),
            weight: 3,
            minor_version: 9,
            unique_id: "TEST".to_string(),
            raw_header: None,
            raw_header_params: None,
            section_keys: SectionKeyList::new(),
            raw_extra_streams: HashMap::new(),
        });
        let lib = SchLib {
            store: Rc::new(RefCell::new(store)),
        };

        let origin =
            RecordOrigin::Param(ParamOrigin::new("|RECORD=1|LIBREFERENCE=R1|PARTCOUNT=1|"));
        let pin_origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=2|OWNERINDEX=0|NAME=1|DESIGNATOR=1|",
        ));
        {
            let mut s = lib.store.borrow_mut();
            let parent_id = s.insert_record(RecordNode::new(1, origin));
            let child_id = s.insert_record(RecordNode::new(2, pin_origin));
            s.insert_group(GroupData {
                parent: parent_id,
                children: vec![child_id],
                original_indices: vec![1],
                parent_original_index: None,
                extra_streams: HashMap::new(),
                meta: GroupMeta::SchComponent {
                    lib_ref: "R1".to_string(),
                    description: "Resistor".to_string(),
                    part_count: 1,
                    section_key: String::new(),
                },
            });
        }

        let buf = Cursor::new(Vec::new());
        lib.save(buf).unwrap();
    }

    #[test]
    fn empty_data_stream_returns_default_group() {
        let (parent, children, _) = parse_data_stream_to_group(&[]).unwrap();
        assert_eq!(parent.key, 1);
        assert!(children.is_empty());
    }

    #[test]
    fn sanitize_cfb_name_replaces_slashes() {
        assert_eq!(sanitize_cfb_name("A/B/C"), "A_B_C");
        assert_eq!(sanitize_cfb_name("simple"), "simple");
    }

    // -----------------------------------------------------------------------
    // DocumentQuery<SchComponent> for SchLib
    // -----------------------------------------------------------------------

    #[test]
    fn schlib_query_component() {
        use crate::v2::handles::SchComponent;
        use crate::v2::traits::DocumentQuery;

        let lib = make_lib_with_components(&["R1", "R2", "C1"]);

        let handle = DocumentQuery::<SchComponent>::query(&lib, "C1").unwrap();
        let lib_ref = handle.lib_ref();
        assert_eq!(lib_ref, "C1");
    }

    #[test]
    fn schlib_query_all_components() {
        use crate::v2::handles::SchComponent;
        use crate::v2::traits::DocumentQuery;

        let lib = make_lib_with_components(&["R1", "R2", "R3"]);

        let results = DocumentQuery::<SchComponent>::query_all(&lib, "R*").unwrap();
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn schlib_query_no_match() {
        use crate::v2::handles::SchComponent;
        use crate::v2::traits::DocumentQuery;

        let lib = make_lib_with_components(&["R1"]);

        let result = DocumentQuery::<SchComponent>::query(&lib, "C1");
        assert!(matches!(result, Err(crate::error::AltiumError::NoMatch(_))));
    }

    #[test]
    fn schlib_query_ambiguous() {
        use crate::v2::handles::SchComponent;
        use crate::v2::traits::DocumentQuery;

        let lib = make_lib_with_components(&["R1", "R2"]);

        let result = DocumentQuery::<SchComponent>::query(&lib, "R*");
        assert!(matches!(
            result,
            Err(crate::error::AltiumError::AmbiguousMatch(2, _))
        ));
    }

    #[test]
    fn schlib_query_modifies_via_handle() {
        use crate::v2::handles::SchComponent;
        use crate::v2::newtypes::LibReference;
        use crate::v2::traits::DocumentQuery;

        let lib = make_lib_with_components(&["R1"]);

        let handle = DocumentQuery::<SchComponent>::query(&lib, "R1").unwrap();
        let mut rec = handle.read();
        rec.set_lib_reference(LibReference::from("R_MODIFIED"));
        handle.write(rec);

        // Verify the record is now dirty
        let store = lib.store.borrow();
        let group_id = store.group_ids()[0];
        let group = store.group(group_id);
        let node = store.record(group.parent_id());
        assert!(node.is_dirty());
    }

    // -----------------------------------------------------------------------
    // Deep queries (SchLib + SchPin)
    // -----------------------------------------------------------------------

    fn make_lib_with_pins(comp_names: &[&str], pin_count: usize) -> SchLib {
        let store = DocumentStore::new(DocumentMeta::SchLib {
            header_text: String::new(),
            weight: 0,
            minor_version: 0,
            unique_id: String::new(),
            raw_header: None,
            raw_header_params: None,
            section_keys: SectionKeyList::new(),
            raw_extra_streams: HashMap::new(),
        });
        let lib = SchLib {
            store: Rc::new(RefCell::new(store)),
        };

        for comp_name in comp_names {
            let comp_str = format!("|RECORD=1|DESIGNATOR={}|", comp_name);
            let comp_origin = RecordOrigin::Param(ParamOrigin::new(&comp_str));
            let comp_node = RecordNode::new(1, comp_origin);

            let mut s = lib.store.borrow_mut();
            let parent_id = s.insert_record(comp_node);

            let mut child_ids = Vec::new();
            for i in 0..pin_count {
                let pin_str = format!("|RECORD=2|Name=PIN{}|Designator={}|", i + 1, i + 1);
                let pin_origin = RecordOrigin::Param(ParamOrigin::new(&pin_str));
                let pin_node = RecordNode::new(2, pin_origin);
                let id = s.insert_record(pin_node);
                child_ids.push(id);
            }

            let original_indices: Vec<usize> = (1..=pin_count).collect();
            s.insert_group(GroupData {
                parent: parent_id,
                children: child_ids,
                original_indices,
                parent_original_index: None,
                extra_streams: HashMap::new(),
                meta: GroupMeta::SchComponent {
                    lib_ref: comp_name.to_string(),
                    description: String::new(),
                    part_count: 1,
                    section_key: String::new(),
                },
            });
        }

        lib
    }

    #[test]
    fn schlib_deep_query_pin() {
        use crate::v2::handles::SchPin;

        let lib = make_lib_with_pins(&["U1"], 2);

        let handle = lib.query_child::<SchPin>("pin[designator=1]").unwrap();
        let rec = handle.read();
        assert_eq!(&*rec.name(), "PIN1");
    }

    #[test]
    fn schlib_deep_query_all_pins() {
        use crate::v2::handles::SchPin;

        let lib = make_lib_with_pins(&["U1", "U2"], 2);

        let results = lib.query_all_children::<SchPin>("pin").unwrap();
        assert_eq!(results.len(), 4);
    }

    #[test]
    fn component_count() {
        let lib = make_lib_with_components(&["A", "B", "C"]);
        assert_eq!(lib.component_count(), 3);
    }

    #[test]
    fn component_names() {
        let lib = make_lib_with_components(&["R1", "C1"]);
        let names = lib.component_names();
        assert!(names.contains(&"R1".to_string()));
        assert!(names.contains(&"C1".to_string()));
    }

    #[test]
    fn find_component_case_insensitive() {
        let lib = make_lib_with_components(&["MyComp"]);
        assert!(lib.find_component("mycomp").is_some());
        assert!(lib.find_component("MISSING").is_none());
    }

    #[test]
    fn empty_unique_id_uses_content_hash_for_document_id() {
        fn make_min_schlib_bytes(extra: &str) -> Vec<u8> {
            let mut cfb = cfb::CompoundFile::create(Cursor::new(Vec::new())).unwrap();
            {
                let mut header = cfb.create_stream("/FileHeader").unwrap();
                let text = format!(
                    "|HEADER=Test|WEIGHT=0|MINORVERSION=0|UNIQUEID=|COMPCOUNT=0|{}|",
                    extra
                );
                use std::io::Write;
                header.write_all(text.as_bytes()).unwrap();
            }
            cfb.flush().unwrap();
            cfb.into_inner().into_inner()
        }

        let bytes_a = make_min_schlib_bytes("A=1");
        let bytes_b = make_min_schlib_bytes("A=2|EXTRA=LONGER");
        assert_ne!(bytes_a, bytes_b, "Fixture bytes should differ");

        let a = SchLib::open(Cursor::new(bytes_a)).unwrap();
        let b = SchLib::open(Cursor::new(bytes_b)).unwrap();

        let did_a = a.document_id().unwrap();
        let did_b = b.document_id().unwrap();
        assert_ne!(did_a.as_str(), did_b.as_str());
    }
}
