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

use std::io::{Cursor, Read, Seek, Write};

use serde::{Deserialize, Serialize};

use crate::error::{AltiumError, Result};
use crate::v2::backing_store::{
    ComponentGroup, ParamOrigin, RecordNode, RecordOrigin,
};
use crate::v2::parameters::ParameterCollection;

use super::section_keys::SectionKeyList;

// Stream name constants
const STREAM_FILE_HEADER: &str = "FileHeader";
const STREAM_SECTION_KEYS: &str = "SectionKeys";
const STREAM_DATA: &str = "Data";

// Size flag mask: low 24 bits = length, bit 24+ = binary mode flag
const SIZE_FLAG_MASK: u32 = 0x00FF_FFFF;

/// SchLib header info.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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
    #[serde(skip)]
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
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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

/// A parsed SchLib library using the v2 backing-store architecture.
///
/// Preserves raw data for unmodified records to enable identity write-back.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SchLib {
    /// Library header metadata.
    pub header: SchLibHeader,
    /// Component groups (one per component, each with its child records).
    pub groups: Vec<ComponentGroup>,
    /// Component entries from the FileHeader (name, description, part count).
    pub component_entries: Vec<SchLibComponentEntry>,
    /// Section key mappings for long component names.
    #[serde(skip)]
    pub section_keys: SectionKeyList,
}

impl SchLib {
    /// Open a SchLib from a reader (CFB compound file).
    pub fn open<R: Read + Seek>(reader: R) -> Result<Self> {
        let mut cfb = cfb::CompoundFile::open(reader)
            .map_err(|e| AltiumError::Cfb(format!("Failed to open CFB: {}", e)))?;

        let mut lib = SchLib::default();

        // 1. Read FileHeader
        lib.header = read_file_header(&mut cfb, &mut lib.component_entries)?;

        // 2. Read SectionKeys
        lib.section_keys = read_section_keys(&mut cfb)?;

        // 3. Read Data stream for each component
        for entry in &lib.component_entries {
            let safe_name = sanitize_cfb_name(&entry.lib_ref);
            let section_key = lib.section_keys.get_key(&safe_name).to_string();
            let data_path = format!("/{}/{}", section_key, STREAM_DATA);

            let group = if let Ok(mut stream) = cfb.open_stream(&data_path) {
                let mut data = Vec::new();
                stream.read_to_end(&mut data).map_err(AltiumError::Io)?;
                parse_data_stream_to_group(&data)?
            } else {
                // Empty component — create a minimal component record
                let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=1|"));
                ComponentGroup::new(RecordNode::new(1, origin), Vec::new(), Vec::new())
            };

            lib.groups.push(group);
        }

        Ok(lib)
    }

    /// Open a SchLib from a file path.
    pub fn open_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let file = std::fs::File::open(path).map_err(AltiumError::Io)?;
        Self::open(file)
    }

    /// Save the SchLib to a writer (creates a new CFB compound file).
    pub fn save<W: Read + Write + Seek>(&self, writer: W) -> Result<()> {
        let mut cfb = cfb::CompoundFile::create(writer)
            .map_err(|e| AltiumError::Cfb(format!("Failed to create CFB: {}", e)))?;

        // 1. Build section keys
        let mut section_keys = SectionKeyList::new();
        for entry in &self.component_entries {
            let safe = sanitize_cfb_name(&entry.lib_ref);
            section_keys.add_key(&safe, 30);
        }

        // 2. Write FileHeader
        if let Some(raw) = &self.header.raw {
            let mut stream = cfb
                .create_stream(format!("/{}", STREAM_FILE_HEADER))
                .map_err(|e| {
                    AltiumError::Cfb(format!("Failed to create FileHeader: {}", e))
                })?;
            stream.write_all(raw).map_err(AltiumError::Io)?;
        } else {
            write_file_header(&mut cfb, &self.header, &self.component_entries)?;
        }

        // 3. Write SectionKeys
        write_section_keys(&mut cfb, &section_keys)?;

        // 4. Write Data stream for each component
        for (i, group) in self.groups.iter().enumerate() {
            if i >= self.component_entries.len() {
                break;
            }
            let safe_name = sanitize_cfb_name(&self.component_entries[i].lib_ref);
            let section_key = section_keys.get_key(&safe_name).to_string();

            let storage_path = format!("/{}", section_key);
            cfb.create_storage(&storage_path).map_err(|e| {
                AltiumError::Cfb(format!("Failed to create storage: {}", e))
            })?;

            let data = build_data_stream_from_group(group)?;
            let data_path = format!("/{}/{}", section_key, STREAM_DATA);
            let mut stream = cfb.create_stream(&data_path).map_err(|e| {
                AltiumError::Cfb(format!("Failed to create Data stream: {}", e))
            })?;
            stream.write_all(&data).map_err(AltiumError::Io)?;
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

    /// Returns a reference to the component groups.
    pub fn components(&self) -> &[ComponentGroup] {
        &self.groups
    }

    /// Returns the number of components in the library.
    pub fn component_count(&self) -> usize {
        self.groups.len()
    }

    /// Returns the library reference names of all components.
    pub fn component_names(&self) -> Vec<&str> {
        self.component_entries
            .iter()
            .map(|e| e.lib_ref.as_str())
            .collect()
    }

    /// Returns the component entry metadata.
    pub fn entries(&self) -> &[SchLibComponentEntry] {
        &self.component_entries
    }

    /// Returns the library header.
    pub fn header(&self) -> &SchLibHeader {
        &self.header
    }

    /// Returns a mutable reference to the library header.
    pub fn header_mut(&mut self) -> &mut SchLibHeader {
        &mut self.header
    }

    /// Iterate all components with entry metadata and a mutable view.
    pub fn for_each_component<F>(&mut self, mut f: F)
    where
        F: FnMut(&SchLibComponentEntry, crate::v2::views::SchComponentView<'_>),
    {
        let entries = &self.component_entries;
        let groups = &mut self.groups;
        for (entry, group) in entries.iter().zip(groups.iter_mut()) {
            let (comp, children) = group.split_borrow();
            let view = crate::v2::views::SchComponentView::new(comp, children);
            f(entry, view);
        }
    }

    /// Access a specific component by index.
    pub fn with_component<R>(
        &mut self,
        index: usize,
        f: impl FnOnce(&SchLibComponentEntry, crate::v2::views::SchComponentView<'_>) -> R,
    ) -> Option<R> {
        if index >= self.groups.len() || index >= self.component_entries.len() {
            return None;
        }
        let entry = &self.component_entries[index];
        let group = &mut self.groups[index];
        let (comp, children) = group.split_borrow();
        let view = crate::v2::views::SchComponentView::new(comp, children);
        Some(f(entry, view))
    }

    /// Find a component by name (case-insensitive), returns index.
    pub fn find_component(&self, name: &str) -> Option<usize> {
        let name_lower = name.to_lowercase();
        self.component_entries
            .iter()
            .position(|e| e.lib_ref.to_lowercase() == name_lower)
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
    // Use add() with string conversion since add_int skips zero values,
    // and CompCount=0 is a valid state.
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

/// Parse a data stream into a ComponentGroup.
///
/// The first record is the component (RECORD=1); remaining records are children.
fn parse_data_stream_to_group(data: &[u8]) -> Result<ComponentGroup> {
    let records = parse_data_stream(data)?;

    if records.is_empty() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=1|"));
        return Ok(ComponentGroup::new(
            RecordNode::new(1, origin),
            Vec::new(),
            Vec::new(),
        ));
    }

    let mut iter = records.into_iter();
    let component = iter.next().unwrap();
    let children: Vec<RecordNode> = iter.collect();
    let original_indices: Vec<usize> = (1..=children.len()).collect();

    Ok(ComponentGroup::new(component, children, original_indices))
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
            // Binary record
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
            // Text (param) record
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
            // Store the raw record bytes (without length header) as snapshot
            node.original_snapshot = record_data;
            records.push(node);
        }
    }

    Ok(records)
}

/// Build a data stream from a ComponentGroup.
fn build_data_stream_from_group(group: &ComponentGroup) -> Result<Vec<u8>> {
    let mut output = Vec::new();

    // Write component record
    write_record_to_stream(&mut output, &group.component)?;

    // Write child records
    for child in &group.children {
        write_record_to_stream(&mut output, child)?;
    }

    Ok(output)
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

    #[test]
    fn data_stream_roundtrip() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=1|LIBREFERENCE=LM358|PARTCOUNT=2|",
        ));
        let component = RecordNode::new(1, origin);
        let pin_origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=2|OWNERINDEX=0|NAME=VCC|",
        ));
        let pin = RecordNode::new(2, pin_origin);
        let group = ComponentGroup::new(component, vec![pin], vec![1]);

        let data = build_data_stream_from_group(&group).unwrap();
        let parsed = parse_data_stream_to_group(&data).unwrap();

        assert_eq!(parsed.component.key, 1);
        assert_eq!(parsed.children.len(), 1);
        assert_eq!(parsed.children[0].key, 2);
    }

    #[test]
    fn cfb_roundtrip() {
        let mut lib = SchLib::default();
        lib.header = SchLibHeader {
            header_text: "Test".to_string(),
            weight: 3,
            minor_version: 9,
            unique_id: "TEST".to_string(),
            raw: None,
        };
        lib.component_entries.push(SchLibComponentEntry {
            lib_ref: "R1".to_string(),
            description: "Resistor".to_string(),
            part_count: 1,
        });
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=1|LIBREFERENCE=R1|PARTCOUNT=1|",
        ));
        let pin_origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=2|OWNERINDEX=0|NAME=1|DESIGNATOR=1|",
        ));
        lib.groups.push(ComponentGroup::new(
            RecordNode::new(1, origin),
            vec![RecordNode::new(2, pin_origin)],
            vec![1],
        ));

        let buf = Cursor::new(Vec::new());
        lib.save(buf).unwrap();
    }

    #[test]
    fn empty_data_stream_returns_default_group() {
        let group = parse_data_stream_to_group(&[]).unwrap();
        assert_eq!(group.component.key, 1);
        assert!(group.children.is_empty());
    }

    #[test]
    fn sanitize_cfb_name_replaces_slashes() {
        assert_eq!(sanitize_cfb_name("A/B/C"), "A_B_C");
        assert_eq!(sanitize_cfb_name("simple"), "simple");
    }
}
