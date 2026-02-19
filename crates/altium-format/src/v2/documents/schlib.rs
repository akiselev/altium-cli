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

use std::collections::HashMap;
use std::io::{Cursor, Read, Seek, Write};
use std::rc::Rc;
use std::cell::RefCell;

use crate::error::{AltiumError, Result};
use crate::v2::backing_store::{ParamOrigin, RecordNode, RecordOrigin};
use crate::v2::ids::GroupId;
use crate::v2::parameters::ParameterCollection;
use crate::v2::store::{DocRef, DocumentMeta, DocumentStore, GroupData, GroupMeta};
use crate::v2::traits::HandleFamily;

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
}

/// A parsed SchLib library using the v2 DocumentStore architecture.
///
/// Preserves raw data for unmodified records to enable identity write-back.
pub struct SchLib {
    store: DocRef,
}

impl SchLib {
    /// Returns a reference to the underlying document store.
    pub fn store(&self) -> &DocRef {
        &self.store
    }

    /// Open a SchLib from a reader (CFB compound file).
    pub fn open<R: Read + Seek>(reader: R) -> Result<Self> {
        let mut cfb = cfb::CompoundFile::open(reader)
            .map_err(|e| AltiumError::Cfb(format!("Failed to open CFB: {}", e)))?;

        // 1. Read FileHeader
        let mut component_entries: Vec<SchLibComponentEntry> = Vec::new();
        let header = read_file_header(&mut cfb, &mut component_entries)?;

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
        let store = self.store.borrow();

        // Extract SchLib metadata
        let (
            header_text,
            weight,
            minor_version,
            unique_id,
            raw_header,
            _stored_section_keys,
            raw_extra_streams,
        ) = match &store.meta {
            DocumentMeta::SchLib {
                header_text,
                weight,
                minor_version,
                unique_id,
                raw_header,
                section_keys,
                raw_extra_streams,
            } => (
                header_text.clone(),
                *weight,
                *minor_version,
                unique_id.clone(),
                raw_header.clone(),
                section_keys.clone(),
                raw_extra_streams.clone(),
            ),
            _ => return Err(AltiumError::Cfb("Expected SchLib metadata".to_string())),
        };

        let mut cfb = cfb::CompoundFile::create(writer)
            .map_err(|e| AltiumError::Cfb(format!("Failed to create CFB: {}", e)))?;

        // Collect component entries from group metadata
        let mut component_entries: Vec<SchLibComponentEntry> = Vec::new();
        for &group_id in store.group_ids() {
            let group = store.group(group_id);
            match &group.meta {
                GroupMeta::SchComponent {
                    lib_ref,
                    description,
                    part_count,
                    ..
                } => {
                    component_entries.push(SchLibComponentEntry {
                        lib_ref: lib_ref.clone(),
                        description: description.clone(),
                        part_count: *part_count,
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
        }

        // 2. Write FileHeader
        if let Some(raw) = &raw_header {
            let mut stream = cfb
                .create_stream(format!("/{}", STREAM_FILE_HEADER))
                .map_err(|e| {
                    AltiumError::Cfb(format!("Failed to create FileHeader: {}", e))
                })?;
            stream.write_all(raw).map_err(AltiumError::Io)?;
        } else {
            let header = SchLibHeader {
                header_text,
                weight,
                minor_version,
                unique_id,
                raw: None,
            };
            write_file_header(&mut cfb, &header, &component_entries)?;
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
            cfb.create_storage(&storage_path).map_err(|e| {
                AltiumError::Cfb(format!("Failed to create storage: {}", e))
            })?;

            // Build data stream from store records
            let parent_node = store.record(group.parent_id());
            let mut data_bytes = Vec::new();
            write_record_to_stream(&mut data_bytes, parent_node)?;
            for &child_id in group.child_ids() {
                let child_node = store.record(child_id);
                write_record_to_stream(&mut data_bytes, child_node)?;
            }

            let data_path = format!("/{}/{}", section_key, STREAM_DATA);
            let mut stream = cfb.create_stream(&data_path).map_err(|e| {
                AltiumError::Cfb(format!("Failed to create Data stream: {}", e))
            })?;
            stream.write_all(&data_bytes).map_err(AltiumError::Io)?;

            // Write per-component extra streams
            for (rel_path, extra_data) in &group.extra_streams {
                let full_path = format!("/{}/{}", section_key, rel_path);
                if let Ok(mut stream) = cfb.create_stream(&full_path) {
                    let _ = stream.write_all(extra_data);
                }
            }

            let _ = i; // suppress unused warning
        }

        // 5. Write library-level extra streams
        {
            let mut created_storages: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            let mut sorted_extras: Vec<_> = raw_extra_streams.iter().collect();
            sorted_extras.sort_by_key(|(k, _)| (*k).clone());
            for (rel_path, data) in &sorted_extras {
                let full_path = format!("/{}", rel_path);
                if let Some(slash_pos) = rel_path.find('/') {
                    let parent = &rel_path[..slash_pos];
                    let parent_path = format!("/{}", parent);
                    if created_storages.insert(parent_path.clone()) {
                        let _ = cfb.create_storage(&parent_path);
                    }
                }
                if let Ok(mut stream) = cfb.create_stream(&full_path) {
                    let _ = stream.write_all(data);
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
                    } => Some(SchLibComponentEntry {
                        lib_ref: lib_ref.clone(),
                        description: description.clone(),
                        part_count: *part_count,
                    }),
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
                ..
            } => SchLibHeader {
                header_text: header_text.clone(),
                weight: *weight,
                minor_version: *minor_version,
                unique_id: unique_id.clone(),
                raw: raw_header.clone(),
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
                GroupMeta::SchComponent { lib_ref, .. }
                    if lib_ref.to_lowercase() == name_lower =>
                {
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
    ) {
        let mut builder = crate::v2::builders::ComponentBuilder::new(template);
        build(&mut builder);

        let (component, children) = builder.build();

        // Extract lib_ref and description from the built component record
        let comp_record = crate::v2::records::SchComponentRecord::from_origin(
            component.origin.clone(),
        );
        let lib_ref = comp_record.lib_reference().to_string();
        let description = comp_record.component_description().to_string();

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
            extra_streams: HashMap::new(),
            meta: GroupMeta::SchComponent {
                lib_ref,
                description,
                part_count: 1,
                section_key: String::new(),
            },
        };

        store.insert_group(group_data);
    }
}

// ---------------------------------------------------------------------------
// DocumentQuery<SchComponent> for SchLib
// ---------------------------------------------------------------------------

impl crate::v2::traits::DocumentQuery<crate::v2::handles::SchComponent> for SchLib {
    fn query(
        &self,
        q: &str,
    ) -> crate::error::Result<crate::v2::handles::SchComponentHandle> {
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
            for &child_id in group.child_ids() {
                let node = store.record(child_id);
                if node.key == T::record_id() {
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
                if node.key == T::record_id() {
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
// Internal helpers
// ---------------------------------------------------------------------------

/// Replace characters that are invalid in CFB storage names.
fn sanitize_cfb_name(name: &str) -> String {
    name.replace('/', "_")
}

/// Read and parse the FileHeader stream.
fn read_file_header<F: Read + Seek>(
    cfb: &mut cfb::CompoundFile<F>,
    entries: &mut Vec<SchLibComponentEntry>,
) -> Result<SchLibHeader> {
    let path = format!("/{}", STREAM_FILE_HEADER);
    let mut stream = cfb
        .open_stream(&path)
        .map_err(|e| AltiumError::Cfb(format!("No FileHeader: {}", e)))?;
    let mut data = Vec::new();
    stream.read_to_end(&mut data).map_err(AltiumError::Io)?;

    let text = String::from_utf8_lossy(&data);
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
    };

    let comp_count = params
        .get("CompCount")
        .map(|v| v.as_int_or(0))
        .unwrap_or(0);
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

        entries.push(SchLibComponentEntry {
            lib_ref,
            description,
            part_count,
        });
    }

    Ok(header)
}

/// Read the SectionKeys stream if present.
fn read_section_keys<F: Read + Seek>(
    cfb: &mut cfb::CompoundFile<F>,
) -> Result<SectionKeyList> {
    let mut keys = SectionKeyList::new();
    if let Ok(mut stream) = cfb.open_stream(format!("/{}", STREAM_SECTION_KEYS)) {
        let mut data = Vec::new();
        stream.read_to_end(&mut data).map_err(AltiumError::Io)?;
        let text = String::from_utf8_lossy(&data);
        let params = ParameterCollection::from_string(&text);
        let count = params
            .get("KeyCount")
            .map(|v| v.as_int_or(0))
            .unwrap_or(0);
        for i in 0..count {
            if let (Some(name_val), Some(key_val)) = (
                params.get(&format!("Key{}", i)),
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
) -> Result<()> {
    let mut params = ParameterCollection::new();
    params.add("HEADER", &header.header_text);
    params.add_int("Weight", header.weight);
    params.add_int("MinorVersion", header.minor_version);
    params.add("UniqueID", &header.unique_id);
    params.add("CompCount", &entries.len().to_string());

    for (i, entry) in entries.iter().enumerate() {
        params.add(&format!("LibRef{}", i), &entry.lib_ref);
        params.add(&format!("CompDescr{}", i), &entry.description);
        params.add(&format!("PartCount{}", i), &entry.part_count.to_string());
    }

    let data = params.to_param_string();
    let path = format!("/{}", STREAM_FILE_HEADER);
    let mut stream = cfb.create_stream(&path).map_err(|e| {
        AltiumError::Cfb(format!("Failed to create FileHeader: {}", e))
    })?;
    stream.write_all(data.as_bytes()).map_err(AltiumError::Io)?;
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
        params.add(&format!("Key{}", i), name);
        params.add(&format!("SectionKey{}", i), key);
    }
    let data = params.to_param_string();
    let path = format!("/{}", STREAM_SECTION_KEYS);
    let mut stream = cfb.create_stream(&path).map_err(|e| {
        AltiumError::Cfb(format!("Failed to create SectionKeys: {}", e))
    })?;
    stream.write_all(data.as_bytes()).map_err(AltiumError::Io)?;
    Ok(())
}

/// Parse a data stream into `(parent_node, children, original_indices)`.
///
/// The first record is the component (RECORD=1); remaining records are children.
fn parse_data_stream_to_group(
    data: &[u8],
) -> Result<(RecordNode, Vec<RecordNode>, Vec<usize>)> {
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
            let origin = RecordOrigin::Binary(
                crate::v2::backing_store::BinaryOrigin::new(record_data),
            );
            let mut node = RecordNode::new(record_type, origin);
            node.original_snapshot = full_raw;
            records.push(node);
        } else {
            let param_str = String::from_utf8_lossy(&record_data).to_string();
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
pub(super) fn write_record_to_stream(
    output: &mut Vec<u8>,
    node: &RecordNode,
) -> Result<()> {
    if node.is_dirty() {
        // Re-serialize from origin
        match &node.origin {
            RecordOrigin::Param(p) => {
                let bytes = p.params.to_param_string();
                let len = bytes.len() as u32;
                output.extend_from_slice(&len.to_le_bytes());
                output.extend_from_slice(bytes.as_bytes());
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
            section_keys: SectionKeyList::new(),
            raw_extra_streams: HashMap::new(),
        });
        let lib = SchLib {
            store: Rc::new(RefCell::new(store)),
        };

        for name in names {
            let param_str =
                format!("|RECORD=1|DESIGNATOR={}|LIBREFERENCE={}|", name, name);
            let origin = RecordOrigin::Param(ParamOrigin::new(&param_str));
            let parent = RecordNode::new(1, origin);

            let mut s = lib.store.borrow_mut();
            let parent_id = s.insert_record(parent);
            s.insert_group(GroupData {
                parent: parent_id,
                children: Vec::new(),
                original_indices: Vec::new(),
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
        let pin_origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=2|OWNERINDEX=0|NAME=VCC|",
        ));
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
            section_keys: SectionKeyList::new(),
            raw_extra_streams: HashMap::new(),
        });
        let lib = SchLib {
            store: Rc::new(RefCell::new(store)),
        };

        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=1|LIBREFERENCE=R1|PARTCOUNT=1|",
        ));
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
        use crate::v2::traits::DocumentQuery;
        use crate::v2::handles::SchComponent;

        let lib = make_lib_with_components(&["R1", "R2", "C1"]);

        let handle = DocumentQuery::<SchComponent>::query(&lib, "C1").unwrap();
        let lib_ref = handle.lib_ref();
        assert_eq!(lib_ref, "C1");
    }

    #[test]
    fn schlib_query_all_components() {
        use crate::v2::traits::DocumentQuery;
        use crate::v2::handles::SchComponent;

        let lib = make_lib_with_components(&["R1", "R2", "R3"]);

        let results = DocumentQuery::<SchComponent>::query_all(&lib, "R*").unwrap();
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn schlib_query_no_match() {
        use crate::v2::traits::DocumentQuery;
        use crate::v2::handles::SchComponent;

        let lib = make_lib_with_components(&["R1"]);

        let result = DocumentQuery::<SchComponent>::query(&lib, "C1");
        assert!(matches!(result, Err(crate::error::AltiumError::NoMatch(_))));
    }

    #[test]
    fn schlib_query_ambiguous() {
        use crate::v2::traits::DocumentQuery;
        use crate::v2::handles::SchComponent;

        let lib = make_lib_with_components(&["R1", "R2"]);

        let result = DocumentQuery::<SchComponent>::query(&lib, "R*");
        assert!(matches!(
            result,
            Err(crate::error::AltiumError::AmbiguousMatch(2, _))
        ));
    }

    #[test]
    fn schlib_query_modifies_via_handle() {
        use crate::v2::traits::DocumentQuery;
        use crate::v2::handles::SchComponent;
        use crate::v2::newtypes::LibReference;

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
                let pin_str = format!(
                    "|RECORD=2|Name=PIN{}|Designator={}|",
                    i + 1,
                    i + 1
                );
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
}
