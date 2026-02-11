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

    /// Returns the number of components in the document.
    pub fn component_count(&self) -> usize {
        self.groups.len()
    }

    /// Count all records of a given type across groups and orphans.
    pub fn count_record_type(&self, record_id: u8) -> usize {
        let mut count = 0;
        for group in &self.groups {
            if group.component.key == record_id {
                count += 1;
            }
            count += group.children.iter().filter(|c| c.key == record_id).count();
        }
        count += self
            .orphan_records
            .iter()
            .filter(|r| r.key == record_id)
            .count();
        count
    }

    /// Returns the sheet record (RECORD=31) if present.
    pub fn sheet_record(&self) -> Option<crate::v2::records::SchSheetRecord> {
        use crate::v2::traits::RecordType;
        let id = crate::v2::records::SchSheetRecord::RECORD_ID;
        self.orphan_records
            .iter()
            .find(|r| r.key == id)
            .map(|r| crate::v2::records::SchSheetRecord::from_origin(r.origin.clone()))
    }

    /// Returns the number of orphan records (records not owned by any component).
    pub fn orphan_count(&self) -> usize {
        self.orphan_records.len()
    }

    /// Iterate ALL records of a given type across groups (children only) and orphans.
    ///
    /// Passes a `&RecordNode` for each matching record. The caller can construct
    /// a typed record via `T::from_origin(node.origin.clone())`.
    pub fn for_each_record_of_type(
        &self,
        record_id: u8,
        mut f: impl FnMut(&crate::v2::backing_store::RecordNode),
    ) {
        for group in &self.groups {
            for child in &group.children {
                if child.key == record_id {
                    f(child);
                }
            }
        }
        for orphan in &self.orphan_records {
            if orphan.key == record_id {
                f(orphan);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// DocumentQuery<SchComponent> for SchDoc
// ---------------------------------------------------------------------------

/// A mutable handle to a single matched component in a SchDoc.
pub struct SchDocComponentQueryHandle<'a> {
    groups: &'a mut [ComponentGroup],
    index: usize,
}

impl<'a> SchDocComponentQueryHandle<'a> {
    /// Consume this handle, construct a `SchComponentView`, pass it to the closure.
    pub fn with_mut<R>(
        self,
        f: impl FnOnce(crate::v2::views::SchComponentView<'_>) -> R,
    ) -> R {
        let group = &mut self.groups[self.index];
        let (comp, children) = group.split_borrow();
        let view = crate::v2::views::SchComponentView::new(comp, children);
        f(view)
    }
}

/// Results from a multi-match component query on a SchDoc.
pub struct SchDocComponentQueryResults<'a> {
    groups: &'a mut [ComponentGroup],
    indices: Vec<usize>,
}

impl<'a> SchDocComponentQueryResults<'a> {
    pub fn len(&self) -> usize {
        self.indices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    pub fn for_each_mut(
        self,
        mut f: impl FnMut(crate::v2::views::SchComponentView<'_>),
    ) {
        for idx in self.indices {
            let group = &mut self.groups[idx];
            let (comp, children) = group.split_borrow();
            let view = crate::v2::views::SchComponentView::new(comp, children);
            f(view);
        }
    }
}

impl crate::v2::traits::DocumentQuery<crate::v2::views::SchComponent> for SchDoc {
    type Handle<'a> = SchDocComponentQueryHandle<'a>;
    type Results<'a> = SchDocComponentQueryResults<'a>;

    fn query(
        &mut self,
        q: &str,
    ) -> crate::error::Result<SchDocComponentQueryHandle<'_>> {
        use crate::v2::query::eval::evaluate;
        let parsed = crate::v2::query::parse(q)?;

        let eval_nodes: Vec<_> = self.groups.iter().map(|g| g.component.clone()).collect();
        let matching = evaluate(&parsed, &eval_nodes);

        match matching.len() {
            0 => Err(crate::error::AltiumError::NoMatch(q.to_string())),
            1 => Ok(SchDocComponentQueryHandle {
                groups: &mut self.groups,
                index: matching[0],
            }),
            n => Err(crate::error::AltiumError::AmbiguousMatch(n, q.to_string())),
        }
    }

    fn query_all(
        &mut self,
        q: &str,
    ) -> crate::error::Result<SchDocComponentQueryResults<'_>> {
        use crate::v2::query::eval::evaluate;
        let parsed = crate::v2::query::parse(q)?;

        let eval_nodes: Vec<_> = self.groups.iter().map(|g| g.component.clone()).collect();
        let indices = evaluate(&parsed, &eval_nodes);

        Ok(SchDocComponentQueryResults {
            groups: &mut self.groups,
            indices,
        })
    }
}

// ---------------------------------------------------------------------------
// Deep queries for SchDoc (cross-group child search)
// ---------------------------------------------------------------------------

/// A mutable handle to a child record found via deep query in SchDoc.
pub struct SchDocDeepChildHandle<'a, T: crate::v2::traits::WrapperFamily> {
    groups: &'a mut [ComponentGroup],
    group_index: usize,
    child_index: usize,
    _marker: std::marker::PhantomData<T>,
}

impl<'a, T: crate::v2::traits::LeafViewConstructor> SchDocDeepChildHandle<'a, T> {
    pub fn with_mut<R>(self, f: impl FnOnce(T::View<'_>) -> R) -> R {
        let node = &mut self.groups[self.group_index].children[self.child_index];
        let view = T::make_view(node);
        f(view)
    }
}

/// Results from a deep query in SchDoc.
pub struct SchDocDeepChildResults<'a, T: crate::v2::traits::WrapperFamily> {
    groups: &'a mut [ComponentGroup],
    matches: Vec<(usize, usize)>,
    _marker: std::marker::PhantomData<T>,
}

impl<'a, T: crate::v2::traits::LeafViewConstructor> SchDocDeepChildResults<'a, T> {
    pub fn len(&self) -> usize {
        self.matches.len()
    }

    pub fn is_empty(&self) -> bool {
        self.matches.is_empty()
    }

    pub fn for_each_mut(self, mut f: impl FnMut(T::View<'_>)) {
        for (gi, ci) in self.matches {
            let node = &mut self.groups[gi].children[ci];
            let view = T::make_view(node);
            f(view);
        }
    }
}

impl<T: crate::v2::traits::LeafViewConstructor> crate::v2::traits::DocumentQuery<T> for SchDoc {
    type Handle<'a> = SchDocDeepChildHandle<'a, T>;
    type Results<'a> = SchDocDeepChildResults<'a, T>;

    fn query(&mut self, q: &str) -> crate::error::Result<SchDocDeepChildHandle<'_, T>> {
        use crate::v2::query::eval::evaluate;
        let parsed = crate::v2::query::parse(q)?;

        let mut matches = Vec::new();
        for (gi, group) in self.groups.iter().enumerate() {
            for (ci, child) in group.children.iter().enumerate() {
                if child.key == T::record_id() {
                    let all = std::slice::from_ref(child);
                    if !evaluate(&parsed, all).is_empty() {
                        matches.push((gi, ci));
                    }
                }
            }
        }

        match matches.len() {
            0 => Err(crate::error::AltiumError::NoMatch(q.to_string())),
            1 => {
                let (gi, ci) = matches[0];
                Ok(SchDocDeepChildHandle {
                    groups: &mut self.groups,
                    group_index: gi,
                    child_index: ci,
                    _marker: std::marker::PhantomData,
                })
            }
            n => Err(crate::error::AltiumError::AmbiguousMatch(n, q.to_string())),
        }
    }

    fn query_all(&mut self, q: &str) -> crate::error::Result<SchDocDeepChildResults<'_, T>> {
        use crate::v2::query::eval::evaluate;
        let parsed = crate::v2::query::parse(q)?;

        let mut matches = Vec::new();
        for (gi, group) in self.groups.iter().enumerate() {
            for (ci, child) in group.children.iter().enumerate() {
                if child.key == T::record_id() {
                    let all = std::slice::from_ref(child);
                    if !evaluate(&parsed, all).is_empty() {
                        matches.push((gi, ci));
                    }
                }
            }
        }

        Ok(SchDocDeepChildResults {
            groups: &mut self.groups,
            matches,
            _marker: std::marker::PhantomData,
        })
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

    // -----------------------------------------------------------------------
    // DocumentQuery tests for SchDoc
    // -----------------------------------------------------------------------

    #[test]
    fn schdoc_query_component() {
        use crate::v2::traits::DocumentQuery;

        let mut doc = SchDoc::default();
        doc.groups.push(ComponentGroup::new(
            RecordNode::new(
                1,
                RecordOrigin::Param(ParamOrigin::new("|RECORD=1|DESIGNATOR=U1|")),
            ),
            vec![
                RecordNode::new(
                    2,
                    RecordOrigin::Param(ParamOrigin::new("|RECORD=2|Name=VCC|Designator=1|")),
                ),
            ],
            vec![1],
        ));
        doc.groups.push(ComponentGroup::new(
            RecordNode::new(
                1,
                RecordOrigin::Param(ParamOrigin::new("|RECORD=1|DESIGNATOR=R1|")),
            ),
            vec![],
            vec![],
        ));

        let desig = DocumentQuery::<crate::v2::views::SchComponent>::query(&mut doc, "U1")
            .unwrap()
            .with_mut(|view| view.designator().to_string());
        assert_eq!(desig, "U1");
    }

    #[test]
    fn schdoc_deep_query_pin() {
        use crate::v2::traits::DocumentQuery;

        let mut doc = SchDoc::default();
        doc.groups.push(ComponentGroup::new(
            RecordNode::new(
                1,
                RecordOrigin::Param(ParamOrigin::new("|RECORD=1|DESIGNATOR=U1|")),
            ),
            vec![
                RecordNode::new(
                    2,
                    RecordOrigin::Param(ParamOrigin::new("|RECORD=2|Name=VCC|Designator=1|")),
                ),
                RecordNode::new(
                    2,
                    RecordOrigin::Param(ParamOrigin::new("|RECORD=2|Name=GND|Designator=2|")),
                ),
            ],
            vec![1, 2],
        ));

        let results =
            DocumentQuery::<crate::v2::views::SchPin>::query_all(&mut doc, "pin").unwrap();
        assert_eq!(results.len(), 2);
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
