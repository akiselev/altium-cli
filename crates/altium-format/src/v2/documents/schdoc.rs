//! SchDoc document I/O using the v2 DocumentStore-based architecture.
//!
//! A SchDoc file is a CFB compound file with a single `/FileHeader` stream
//! containing all records as a flat length-prefixed sequence. Records are
//! grouped by OWNERINDEX: component records (RECORD=1) own child records
//! that reference them by index.

use std::collections::HashMap;
use std::io::{Cursor, Read, Seek, Write};

use crate::error::{AltiumError, Result};
use crate::v2::backing_store::{ParamOrigin, RecordNode, RecordOrigin};
use crate::v2::ids::{GroupId, RecordId};
use crate::v2::parameters::ParameterCollection;
use crate::v2::store::{DocRef, DocumentMeta, DocumentStore, GroupData, GroupMeta};

const STREAM_FILE_HEADER: &str = "FileHeader";
const SIZE_FLAG_MASK: u32 = 0x00FF_FFFF;

/// A parsed SchDoc document using the v2 DocumentStore architecture.
///
/// Records are grouped by OWNERINDEX. Component records (RECORD=1) form
/// groups with their children. Records that don't belong to any component
/// are stored as orphans in the shared store.
pub struct SchDoc {
    store: DocRef,
}

impl SchDoc {
    /// Returns a reference to the underlying document store.
    pub fn store(&self) -> &DocRef {
        &self.store
    }

    /// Open a SchDoc from a reader.
    pub fn open<R: Read + Seek>(reader: R) -> Result<Self> {
        let mut cfb = cfb::CompoundFile::open(reader)
            .map_err(|e| AltiumError::Cfb(format!("Failed to open CFB: {}", e)))?;

        let mut stream = cfb
            .open_stream(format!("/{}", STREAM_FILE_HEADER))
            .map_err(|e| AltiumError::Cfb(format!("No FileHeader: {}", e)))?;
        let mut data = Vec::new();
        stream.read_to_end(&mut data).map_err(AltiumError::Io)?;

        let meta = DocumentMeta::SchDoc {
            header_raw: Some(data.clone()),
        };
        let mut doc_store = DocumentStore::new(meta);

        let records = parse_flat_stream(&data)?;
        group_by_owner_index(&mut doc_store, records);

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
        let mut cfb = cfb::CompoundFile::create(writer)
            .map_err(|e| AltiumError::Cfb(format!("Failed to create CFB: {}", e)))?;

        let data = flatten_to_stream(self)?;

        let mut stream = cfb
            .create_stream(format!("/{}", STREAM_FILE_HEADER))
            .map_err(|e| AltiumError::Cfb(format!("Failed to create FileHeader: {}", e)))?;
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

    /// Returns the number of components (groups) in the document.
    pub fn component_count(&self) -> usize {
        self.store.borrow().group_count()
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
    pub fn sheet_record(&self) -> Option<crate::v2::records::SchSheetRecord> {
        use crate::v2::traits::RecordType;
        let id = crate::v2::records::SchSheetRecord::RECORD_ID;
        let store = self.store.borrow();
        store
            .orphan_ids()
            .iter()
            .find(|&&rid| store.record(rid).key == id)
            .map(|&rid| {
                crate::v2::records::SchSheetRecord::from_origin(
                    store.record(rid).origin.clone(),
                )
            })
    }

    /// Returns the number of orphan records (records not owned by any component).
    pub fn orphan_count(&self) -> usize {
        self.store.borrow().orphan_ids().len()
    }

    /// Iterate all records of a given type across group children and orphans.
    ///
    /// Passes a cloned `RecordNode` for each matching record.
    pub fn for_each_record_of_type(
        &self,
        record_id: u8,
        mut f: impl FnMut(&RecordNode),
    ) {
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

impl crate::v2::traits::DocumentQuery<crate::v2::handles::SchComponent> for SchDoc {
    fn query(
        &self,
        q: &str,
    ) -> crate::error::Result<crate::v2::handles::SchComponentHandle> {
        use crate::v2::query::eval::evaluate;
        let parsed = crate::v2::query::parse(q)?;

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
            1 => Ok(crate::v2::handles::SchComponentHandle::new(
                self.store.clone(),
                group_ids[matching[0]],
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
        let group_ids: Vec<GroupId> = store.group_ids().to_vec();

        let eval_nodes: Vec<RecordNode> = group_ids
            .iter()
            .map(|&gid| store.record(store.group(gid).parent_id()).clone())
            .collect();

        let indices = evaluate(&parsed, &eval_nodes);
        drop(store);

        let handles = indices
            .into_iter()
            .map(|i| {
                crate::v2::handles::SchComponentHandle::new(
                    self.store.clone(),
                    group_ids[i],
                )
            })
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
    pub fn query_child<T: crate::v2::traits::HandleFamily>(
        &self,
        q: &str,
    ) -> crate::error::Result<T::Handle> {
        use crate::v2::query::eval::evaluate;
        let parsed = crate::v2::query::parse(q)?;

        let store = self.store.borrow();
        let mut matches: Vec<RecordId> = Vec::new();

        for &gid in store.group_ids() {
            let group = store.group(gid);
            for &cid in group.child_ids() {
                if store.record(cid).key == T::record_id() {
                    let node = store.record(cid);
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
            1 => Ok(T::make_handle(self.store.clone(), matches[0])),
            n => Err(crate::error::AltiumError::AmbiguousMatch(n, q.to_string())),
        }
    }

    /// Query all child records of type `T` across all component groups.
    pub fn query_all_children<T: crate::v2::traits::HandleFamily>(
        &self,
        q: &str,
    ) -> crate::error::Result<Vec<T::Handle>> {
        use crate::v2::query::eval::evaluate;
        let parsed = crate::v2::query::parse(q)?;

        let store = self.store.borrow();
        let mut matches: Vec<RecordId> = Vec::new();

        for &gid in store.group_ids() {
            let group = store.group(gid);
            for &cid in group.child_ids() {
                if store.record(cid).key == T::record_id() {
                    let node = store.record(cid);
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
            .map(|id| T::make_handle(self.store.clone(), id))
            .collect();

        Ok(handles)
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Parse a flat record stream into indexed (flat_index, RecordNode) pairs.
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

    for (_flat_idx, comp_node) in component_records {
        let parent_id = store.insert_record(comp_node);
        let group_data = GroupData {
            parent: parent_id,
            children: Vec::new(),
            original_indices: Vec::new(),
            extra_streams: HashMap::new(),
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
            group_entries[owner_index as usize].2.push((flat_idx, child_id));
        } else {
            let child_id = store.insert_record(node);
            store.orphan_records.push(child_id);
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

/// Flatten the document back to a sequential record stream for writing.
fn flatten_to_stream(doc: &SchDoc) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    let store = doc.store.borrow();

    for &gid in store.group_ids() {
        let group = store.group(gid);
        let parent = store.record(group.parent_id());
        super::schlib::write_record_to_stream(&mut output, parent)?;

        for &cid in group.child_ids() {
            let child = store.record(cid);
            super::schlib::write_record_to_stream(&mut output, child)?;
        }
    }

    for &oid in store.orphan_ids() {
        let orphan = store.record(oid);
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
    use crate::v2::backing_store::RecordOrigin;

    fn make_store_with_records(records: Vec<(usize, RecordNode)>) -> DocumentStore {
        let mut store = DocumentStore::new(DocumentMeta::SchDoc { header_raw: None });
        group_by_owner_index(&mut store, records);
        store
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
                    RecordOrigin::Param(ParamOrigin::new("|RECORD=1|DESIGNATOR=R1|")),
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
                RecordNode::new(
                    34,
                    RecordOrigin::Param(ParamOrigin::new("|RECORD=34|")),
                ),
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
                RecordNode::new(
                    31,
                    RecordOrigin::Param(ParamOrigin::new("|RECORD=31|")),
                ),
            ),
        ];

        let store =
            std::rc::Rc::new(std::cell::RefCell::new(make_store_with_records(records)));
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
                    31,
                    RecordOrigin::Param(ParamOrigin::new("|RECORD=31|")),
                ),
            ),
        ];

        let store =
            std::rc::Rc::new(std::cell::RefCell::new(make_store_with_records(records)));
        let doc = SchDoc { store };

        assert_eq!(doc.count_record_type(1), 1);
        assert_eq!(doc.count_record_type(2), 2);
        assert_eq!(doc.count_record_type(31), 1);
    }

    #[test]
    fn schdoc_query_component() {
        use crate::v2::traits::DocumentQuery;

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

        let store =
            std::rc::Rc::new(std::cell::RefCell::new(make_store_with_records(records)));
        let doc = SchDoc { store };

        let handle =
            DocumentQuery::<crate::v2::handles::SchComponent>::query(&doc, "U1")
                .unwrap();
        let comp = handle.read();
        assert_eq!(&*comp.designator(), "U1");
    }

    #[test]
    fn schdoc_query_all_components() {
        use crate::v2::traits::DocumentQuery;

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

        let store =
            std::rc::Rc::new(std::cell::RefCell::new(make_store_with_records(records)));
        let doc = SchDoc { store };

        let handles =
            DocumentQuery::<crate::v2::handles::SchComponent>::query_all(&doc, "#1")
                .unwrap();
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

        let store =
            std::rc::Rc::new(std::cell::RefCell::new(make_store_with_records(records)));
        let doc = SchDoc { store };

        let handles = doc
            .query_all_children::<crate::v2::handles::SchPin>("pin")
            .unwrap();
        assert_eq!(handles.len(), 2);
    }
}
