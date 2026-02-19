//! Central document store for the v2 ID-handle architecture.
//!
//! [`DocumentStore`] holds all records and groups for a document, accessed
//! via [`RecordId`] and [`GroupId`] keys. Wrapped in `Rc<RefCell<>>` as
//! [`DocRef`] for shared ownership by handles.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use slotmap::SlotMap;

use crate::v2::backing_store::{PcbPrimitiveRef, RecordNode};
use crate::v2::ids::{GroupId, RecordId};

/// Shared reference to a [`DocumentStore`].
pub type DocRef = Rc<RefCell<DocumentStore>>;

/// Central store for all records and groups in a document.
///
/// Handles (e.g. `SchPinHandle`, `SchComponentHandle`) hold a `DocRef` and
/// an ID, reading/writing records through shared `Rc<RefCell<>>` access.
pub struct DocumentStore {
    pub(crate) records: SlotMap<RecordId, RecordNode>,
    pub(crate) groups: SlotMap<GroupId, GroupData>,
    pub(crate) group_order: Vec<GroupId>,
    pub(crate) orphan_records: Vec<RecordId>,
    pub(crate) meta: DocumentMeta,
}

/// Data for a single group (component or footprint).
pub struct GroupData {
    /// The parent record (component or footprint metadata).
    pub(crate) parent: RecordId,
    /// Child record IDs in order.
    pub(crate) children: Vec<RecordId>,
    /// Original indices of the children for round-trip serialization.
    pub(crate) original_indices: Vec<usize>,
    /// Extra CFB streams preserved for round-trip.
    pub(crate) extra_streams: HashMap<String, Vec<u8>>,
    /// Group-type-specific metadata.
    pub(crate) meta: GroupMeta,
}

/// Type-specific metadata for a group.
pub enum GroupMeta {
    SchComponent {
        lib_ref: String,
        description: String,
        part_count: i32,
        section_key: String,
    },
    PcbFootprint {
        name: String,
        raw_pattern_name_block: Vec<u8>,
        original_primitive_order: Vec<PcbPrimitiveRef>,
        raw_header: Vec<u8>,
    },
    SchDocComponent,
}

/// Document-level metadata.
pub enum DocumentMeta {
    SchLib {
        header_text: String,
        weight: i32,
        minor_version: i32,
        unique_id: String,
        raw_header: Option<Vec<u8>>,
        section_keys: crate::v2::documents::section_keys::SectionKeyList,
        raw_extra_streams: HashMap<String, Vec<u8>>,
    },
    SchDoc {
        header_raw: Option<Vec<u8>>,
    },
    PcbLib {
        section_keys: crate::v2::documents::section_keys::SectionKeyList,
        raw_extra_streams: HashMap<String, Vec<u8>>,
    },
    Empty,
}

impl DocumentStore {
    /// Create a new empty store with the given document metadata.
    pub fn new(meta: DocumentMeta) -> Self {
        Self {
            records: SlotMap::with_key(),
            groups: SlotMap::with_key(),
            group_order: Vec::new(),
            orphan_records: Vec::new(),
            meta,
        }
    }

    /// Create a new store wrapped in `Rc<RefCell<>>`.
    pub fn new_ref(meta: DocumentMeta) -> DocRef {
        Rc::new(RefCell::new(Self::new(meta)))
    }

    /// Insert a record, returning its ID.
    pub fn insert_record(&mut self, node: RecordNode) -> RecordId {
        self.records.insert(node)
    }

    /// Insert a group, returning its ID. Also appends to group_order.
    pub fn insert_group(&mut self, data: GroupData) -> GroupId {
        let id = self.groups.insert(data);
        self.group_order.push(id);
        id
    }

    /// Get a shared reference to a record.
    pub fn record(&self, id: RecordId) -> &RecordNode {
        &self.records[id]
    }

    /// Get a mutable reference to a record.
    pub fn record_mut(&mut self, id: RecordId) -> &mut RecordNode {
        &mut self.records[id]
    }

    /// Get a shared reference to a group.
    pub fn group(&self, id: GroupId) -> &GroupData {
        &self.groups[id]
    }

    /// Get a mutable reference to a group.
    pub fn group_mut(&mut self, id: GroupId) -> &mut GroupData {
        &mut self.groups[id]
    }

    /// Get a reference to the document metadata.
    pub fn meta(&self) -> &DocumentMeta {
        &self.meta
    }

    /// Get a mutable reference to the document metadata.
    pub fn meta_mut(&mut self) -> &mut DocumentMeta {
        &mut self.meta
    }

    /// Returns the number of groups.
    pub fn group_count(&self) -> usize {
        self.group_order.len()
    }

    /// Returns the group IDs in order.
    pub fn group_ids(&self) -> &[GroupId] {
        &self.group_order
    }

    /// Returns the orphan record IDs.
    pub fn orphan_ids(&self) -> &[RecordId] {
        &self.orphan_records
    }
}

impl GroupData {
    /// Returns the parent record ID.
    pub fn parent_id(&self) -> RecordId {
        self.parent
    }

    /// Returns the child record IDs.
    pub fn child_ids(&self) -> &[RecordId] {
        &self.children
    }

    /// Returns the group metadata.
    pub fn meta(&self) -> &GroupMeta {
        &self.meta
    }
}
