//! SchDoc document I/O using the v2 backing-store architecture.
//!
//! A SchDoc file is a CFB compound file with a single `/FileHeader` stream
//! containing all records as a flat length-prefixed sequence. Records are
//! grouped by OWNERINDEX: component records (RECORD=1) own child records
//! that reference them by index.

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Seek, Write};

use serde::{Deserialize, Serialize};

use crate::error::{AltiumError, Result};
use crate::v2::backing_store::{
    ComponentGroup, ParamOrigin, RecordNode, RecordOrigin,
};
use crate::v2::parameters::ParameterCollection;

const STREAM_FILE_HEADER: &str = "FileHeader";
const SIZE_FLAG_MASK: u32 = 0x00FF_FFFF;

/// A parsed SchDoc document using the v2 backing-store architecture.
///
/// Records are grouped by OWNERINDEX. Component records (RECORD=1) form
/// groups with their children. Records that don't belong to any component
/// are stored as orphans.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SchDoc {
    /// Component groups (component record + owned children).
    pub groups: Vec<ComponentGroup>,
    /// Records that don't belong to any component group.
    pub orphan_records: Vec<RecordNode>,
    /// Raw bytes of the FileHeader stream (for identity write-back).
    pub header_raw: Option<Vec<u8>>,
}

impl SchDoc {
    /// Open a SchDoc from a reader.
    pub fn open<R: Read + Seek>(reader: R) -> Result<Self> {
        let mut cfb = cfb::CompoundFile::open(reader)
            .map_err(|e| AltiumError::Cfb(format!("Failed to open CFB: {}", e)))?;

        let mut doc = SchDoc::default();

        // Read FileHeader (contains all records as a flat stream)
        let mut stream = cfb
            .open_stream(format!("/{}", STREAM_FILE_HEADER))
            .map_err(|e| AltiumError::Cfb(format!("No FileHeader: {}", e)))?;
        let mut data = Vec::new();
        stream.read_to_end(&mut data).map_err(AltiumError::Io)?;
        doc.header_raw = Some(data.clone());

        // Parse flat record stream
        let records = parse_flat_stream(&data)?;

        // Group records by OWNERINDEX
        group_by_owner_index(&mut doc, records);

        Ok(doc)
    }

    /// Open a SchDoc from a file path.
    pub fn open_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let file = std::fs::File::open(path).map_err(AltiumError::Io)?;
        Self::open(file)
    }

    /// Save a SchDoc to a writer.
    pub fn save<W: Read + Write + Seek>(&self, writer: W) -> Result<()> {
        let mut cfb = cfb::CompoundFile::create(writer)
            .map_err(|e| AltiumError::Cfb(format!("Failed to create CFB: {}", e)))?;

        // Flatten back to original order
        let data = flatten_to_stream(self)?;

        let mut stream = cfb
            .create_stream(format!("/{}", STREAM_FILE_HEADER))
            .map_err(|e| {
                AltiumError::Cfb(format!("Failed to create FileHeader: {}", e))
            })?;
        stream.write_all(&data).map_err(AltiumError::Io)?;

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

    /// Returns the number of components in the document.
    pub fn component_count(&self) -> usize {
        self.groups.len()
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Parse a flat record stream into indexed records.
///
/// Returns `(flat_index, RecordNode)` pairs. The flat index is the position
/// of the record in the original stream (used for OWNERINDEX grouping).
fn parse_flat_stream(data: &[u8]) -> Result<Vec<(usize, RecordNode)>> {
    let mut records = Vec::new();
    let mut cursor = Cursor::new(data);
    let total_len = data.len() as u64;
    let mut index = 0usize;

    while cursor.position() < total_len {
        let mut len_buf = [0u8; 4];
        if Read::read_exact(&mut cursor, &mut len_buf).is_err() {
            break;
        }
        let size_raw = u32::from_le_bytes(len_buf);
        let is_binary = (size_raw & !SIZE_FLAG_MASK) != 0;
        let record_len = (size_raw & SIZE_FLAG_MASK) as usize;

        if record_len == 0 {
            index += 1;
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
            records.push((index, node));
        } else {
            let param_str = String::from_utf8_lossy(&record_data).to_string();
            let params = ParameterCollection::from_string(&param_str);
            let record_id = params
                .get("RECORD")
                .map(|v| v.as_int_or(0) as u8)
                .unwrap_or(0);

            if record_id == 0 {
                index += 1;
                continue;
            }

            let origin = RecordOrigin::Param(ParamOrigin::new(&param_str));
            let mut node = RecordNode::new(record_id, origin);
            node.original_snapshot = record_data;
            records.push((index, node));
        }
        index += 1;
    }

    Ok(records)
}

/// Group records by OWNERINDEX into ComponentGroups.
///
/// Component records (RECORD=1) form group parents. Other records are assigned
/// to the component whose group-order index matches their OWNERINDEX value.
/// Records with no valid owner become orphans.
fn group_by_owner_index(doc: &mut SchDoc, records: Vec<(usize, RecordNode)>) {
    // Separate components from children
    let mut component_records = Vec::new();
    let mut child_records = Vec::new();

    for (flat_idx, node) in records {
        if node.key == 1 {
            // Component record
            component_records.push((flat_idx, node));
        } else {
            child_records.push((flat_idx, node));
        }
    }

    // Initialize groups: each component starts with empty children
    let mut groups: Vec<(usize, RecordNode, Vec<(usize, RecordNode)>)> = Vec::new();
    let mut _component_positions: BTreeMap<usize, usize> = BTreeMap::new();

    for (flat_idx, comp) in component_records {
        _component_positions.insert(flat_idx, groups.len());
        groups.push((flat_idx, comp, Vec::new()));
    }

    // Assign children to groups by OWNERINDEX
    let mut orphans: Vec<RecordNode> = Vec::new();

    for (flat_idx, node) in child_records {
        let owner_index = match &node.origin {
            RecordOrigin::Param(p) => {
                p.params.get("OWNERINDEX").map(|v| v.as_int_or(-1)).unwrap_or(-1)
            }
            _ => -1,
        };

        if owner_index >= 0 && (owner_index as usize) < groups.len() {
            groups[owner_index as usize].2.push((flat_idx, node));
        } else {
            orphans.push(node);
        }
    }

    // Convert to ComponentGroups
    for (_comp_idx, comp, children) in groups {
        let original_indices: Vec<usize> =
            children.iter().map(|(idx, _)| *idx).collect();
        let child_nodes: Vec<RecordNode> =
            children.into_iter().map(|(_, n)| n).collect();
        doc.groups
            .push(ComponentGroup::new(comp, child_nodes, original_indices));
    }

    doc.orphan_records = orphans;
}

/// Flatten the document back to a sequential record stream for writing.
fn flatten_to_stream(doc: &SchDoc) -> Result<Vec<u8>> {
    let mut output = Vec::new();

    for group in &doc.groups {
        super::schlib::write_record_to_stream(&mut output, &group.component)?;
        for child in &group.children {
            super::schlib::write_record_to_stream(&mut output, child)?;
        }
    }

    for orphan in &doc.orphan_records {
        super::schlib::write_record_to_stream(&mut output, orphan)?;
    }

    Ok(output)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_index_grouping() {
        let records = vec![
            (
                0,
                RecordNode::new(
                    1,
                    RecordOrigin::Param(ParamOrigin::new(
                        "|RECORD=1|DESIGNATOR=U1|",
                    )),
                ),
            ),
            (
                1,
                RecordNode::new(
                    2,
                    RecordOrigin::Param(ParamOrigin::new(
                        "|RECORD=2|OWNERINDEX=0|NAME=VCC|",
                    )),
                ),
            ),
            (
                2,
                RecordNode::new(
                    2,
                    RecordOrigin::Param(ParamOrigin::new(
                        "|RECORD=2|OWNERINDEX=0|NAME=GND|",
                    )),
                ),
            ),
            (
                3,
                RecordNode::new(
                    1,
                    RecordOrigin::Param(ParamOrigin::new(
                        "|RECORD=1|DESIGNATOR=R1|",
                    )),
                ),
            ),
            (
                4,
                RecordNode::new(
                    2,
                    RecordOrigin::Param(ParamOrigin::new(
                        "|RECORD=2|OWNERINDEX=1|NAME=1|",
                    )),
                ),
            ),
        ];

        let mut doc = SchDoc::default();
        group_by_owner_index(&mut doc, records);

        assert_eq!(doc.groups.len(), 2);
        assert_eq!(doc.groups[0].children.len(), 2); // U1 has 2 pins
        assert_eq!(doc.groups[1].children.len(), 1); // R1 has 1 pin
    }

    #[test]
    fn orphan_records_collected() {
        let records = vec![
            (
                0,
                RecordNode::new(
                    1,
                    RecordOrigin::Param(ParamOrigin::new(
                        "|RECORD=1|DESIGNATOR=U1|",
                    )),
                ),
            ),
            (
                1,
                RecordNode::new(
                    34, // sheet record (no OWNERINDEX)
                    RecordOrigin::Param(ParamOrigin::new("|RECORD=34|")),
                ),
            ),
        ];

        let mut doc = SchDoc::default();
        group_by_owner_index(&mut doc, records);

        assert_eq!(doc.groups.len(), 1);
        assert_eq!(doc.orphan_records.len(), 1);
        assert_eq!(doc.orphan_records[0].key, 34);
    }

    #[test]
    fn empty_stream_produces_empty_doc() {
        let records: Vec<(usize, RecordNode)> = Vec::new();
        let mut doc = SchDoc::default();
        group_by_owner_index(&mut doc, records);

        assert!(doc.groups.is_empty());
        assert!(doc.orphan_records.is_empty());
    }
}
