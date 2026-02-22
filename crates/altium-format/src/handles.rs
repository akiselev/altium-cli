//! Handle types for the v2 ID-handle architecture.
//!
//! Handles are thin `Clone` types holding a [`DocRef`] and a record/group ID.
//! They provide `read()` and `write()` methods for record access without
//! closures or borrow-carrying views.

use crate::ids::{GroupId, RecordId};
use crate::store::DocRef;
use crate::traits::{FromOrigin, HandleFamily, RecordType};

// ---------------------------------------------------------------------------
// Record handle macro
// ---------------------------------------------------------------------------

macro_rules! impl_record_handle {
    ($handle:ident, $record:ty) => {
        #[derive(Clone)]
        pub struct $handle {
            pub(crate) store: DocRef,
            pub(crate) id: RecordId,
        }

        impl $handle {
            pub(crate) fn new(store: DocRef, id: RecordId) -> Self {
                Self { store, id }
            }

            pub fn id(&self) -> RecordId {
                self.id
            }

            pub fn semantic_id(&self) -> Option<crate::semantic_ids::SemanticId> {
                let mut store = self.store.borrow_mut();
                store.ensure_semantic_ids();
                store.record_semantic_id(self.id).cloned()
            }

            /// Returns `true` if this record's backing origin is binary.
            pub fn is_binary_origin(&self) -> bool {
                let store = self.store.borrow();
                store.records[self.id].origin.is_binary()
            }

            pub fn read(&self) -> $record {
                let store = self.store.borrow();
                let node = &store.records[self.id];
                <$record as FromOrigin>::from_origin(node.origin.clone())
            }

            pub fn write(&self, record: $record) {
                let mut store = self.store.borrow_mut();
                let node = &mut store.records[self.id];
                node.origin = <$record as FromOrigin>::into_origin(record);
                node.mark_dirty();
                store.mark_semantic_ids_dirty();
            }
        }
    };
}

// ---------------------------------------------------------------------------
// Handle family macro
// ---------------------------------------------------------------------------

macro_rules! impl_handle_family {
    ($marker:ident, $record:ty, $handle:ty) => {
        pub enum $marker {}
        impl HandleFamily for $marker {
            type Record = $record;
            type Handle = $handle;

            fn try_make_handle(store: DocRef, id: RecordId) -> crate::error::Result<Self::Handle> {
                Ok(<$handle>::new(store, id))
            }
        }
    };
}

// ---------------------------------------------------------------------------
// Schematic record handles
// ---------------------------------------------------------------------------

use crate::records::{
    SchArcRecord, SchBezierRecord, SchBlanketRecord, SchBusEntryRecord, SchBusRecord,
    SchComponentRecord, SchDesignatorRecord, SchEllipseRecord, SchEllipticalArcRecord,
    SchImageRecord, SchImplementationListRecord, SchImplementationParametersRecord,
    SchImplementationRecord, SchJunctionRecord, SchLabelRecord, SchLineRecord,
    SchMapDefinerListRecord, SchMapDefinerRecord, SchNetLabelRecord, SchNoERCRecord, SchNoteRecord,
    SchParameterRecord, SchPieRecord, SchPinRecord, SchPolygonRecord, SchPolylineRecord,
    SchPortRecord, SchPowerRecord, SchRectangleRecord, SchRoundRectangleRecord,
    SchSheetEntryRecord, SchSheetFileNameRecord, SchSheetNameRecord, SchSheetRecord,
    SchSheetSymbolRecord, SchSymbolRecord, SchTaskHolderRecord, SchTextFrameRecord, SchWireRecord,
};

impl_record_handle!(SchPinHandle, SchPinRecord);
impl_record_handle!(SchArcHandle, SchArcRecord);
impl_record_handle!(SchLineHandle, SchLineRecord);
impl_record_handle!(SchRectangleHandle, SchRectangleRecord);
impl_record_handle!(SchBezierHandle, SchBezierRecord);
impl_record_handle!(SchPolylineHandle, SchPolylineRecord);
impl_record_handle!(SchPolygonHandle, SchPolygonRecord);
impl_record_handle!(SchEllipseHandle, SchEllipseRecord);
impl_record_handle!(SchPieHandle, SchPieRecord);
impl_record_handle!(SchRoundRectangleHandle, SchRoundRectangleRecord);
impl_record_handle!(SchEllipticalArcHandle, SchEllipticalArcRecord);
impl_record_handle!(SchImageHandle, SchImageRecord);
impl_record_handle!(SchDesignatorHandle, SchDesignatorRecord);
impl_record_handle!(SchParameterHandle, SchParameterRecord);
impl_record_handle!(SchSymbolHandle, SchSymbolRecord);
impl_record_handle!(SchLabelHandle, SchLabelRecord);
impl_record_handle!(SchPowerHandle, SchPowerRecord);
impl_record_handle!(SchPortHandle, SchPortRecord);
impl_record_handle!(SchNoERCHandle, SchNoERCRecord);
impl_record_handle!(SchNetLabelHandle, SchNetLabelRecord);
impl_record_handle!(SchBusHandle, SchBusRecord);
impl_record_handle!(SchWireHandle, SchWireRecord);
impl_record_handle!(SchTextFrameHandle, SchTextFrameRecord);
impl_record_handle!(SchJunctionHandle, SchJunctionRecord);
impl_record_handle!(SchSheetHandle, SchSheetRecord);
impl_record_handle!(SchSheetNameHandle, SchSheetNameRecord);
impl_record_handle!(SchSheetFileNameHandle, SchSheetFileNameRecord);
impl_record_handle!(SchBusEntryHandle, SchBusEntryRecord);
impl_record_handle!(SchSheetSymbolHandle, SchSheetSymbolRecord);
impl_record_handle!(SchSheetEntryHandle, SchSheetEntryRecord);
impl_record_handle!(SchImplementationListHandle, SchImplementationListRecord);
impl_record_handle!(SchImplementationHandle, SchImplementationRecord);
impl_record_handle!(SchMapDefinerListHandle, SchMapDefinerListRecord);
impl_record_handle!(SchMapDefinerHandle, SchMapDefinerRecord);
impl_record_handle!(
    SchImplementationParametersHandle,
    SchImplementationParametersRecord
);
impl_record_handle!(SchNoteHandle, SchNoteRecord);
impl_record_handle!(SchBlanketHandle, SchBlanketRecord);
impl_record_handle!(SchTaskHolderHandle, SchTaskHolderRecord);

impl SchPinHandle {
    /// Read a pin record regardless of whether its backing origin is param
    /// text or legacy binary.
    pub fn read_normalized(&self) -> SchPinRecord {
        let store = self.store.borrow();
        let node = &store.records[self.id];
        match &node.origin {
            crate::backing_store::RecordOrigin::Param(_) => {
                SchPinRecord::from_origin(node.origin.clone())
            }
            crate::backing_store::RecordOrigin::Binary(b) => {
                SchPinRecord::from_legacy_binary_record_data(&b.raw_block).unwrap_or_else(|| {
                    panic!(
                        "failed to decode legacy binary pin payload ({} bytes)",
                        b.raw_block.len()
                    )
                })
            }
        }
    }
}

// ---------------------------------------------------------------------------
// PCB record handles
// ---------------------------------------------------------------------------

use crate::records::{
    PcbArcRecord, PcbComponentBodyRecord, PcbConnectionRecord, PcbFillRecord, PcbFootprintRecord,
    PcbPadRecord, PcbRegionRecord, PcbTextRecord, PcbTrackRecord, PcbViaRecord,
};

impl_record_handle!(PcbTrackHandle, PcbTrackRecord);
impl_record_handle!(PcbArcHandle, PcbArcRecord);
impl_record_handle!(PcbConnectionHandle, PcbConnectionRecord);
impl_record_handle!(PcbFillHandle, PcbFillRecord);
impl_record_handle!(PcbPadHandle, PcbPadRecord);
impl_record_handle!(PcbViaHandle, PcbViaRecord);
impl_record_handle!(PcbTextHandle, PcbTextRecord);
impl_record_handle!(PcbRegionHandle, PcbRegionRecord);
impl_record_handle!(PcbComponentBodyHandle, PcbComponentBodyRecord);
impl_record_handle!(PcbFootprintMetadataHandle, PcbFootprintRecord);

// ---------------------------------------------------------------------------
// Schematic handle families
// ---------------------------------------------------------------------------

impl_handle_family!(SchPin, SchPinRecord, SchPinHandle);
impl_handle_family!(SchArc, SchArcRecord, SchArcHandle);
impl_handle_family!(SchLine, SchLineRecord, SchLineHandle);
impl_handle_family!(SchRectangle, SchRectangleRecord, SchRectangleHandle);
impl_handle_family!(SchBezier, SchBezierRecord, SchBezierHandle);
impl_handle_family!(SchPolyline, SchPolylineRecord, SchPolylineHandle);
impl_handle_family!(SchPolygon, SchPolygonRecord, SchPolygonHandle);
impl_handle_family!(SchEllipse, SchEllipseRecord, SchEllipseHandle);
impl_handle_family!(SchPie, SchPieRecord, SchPieHandle);
impl_handle_family!(
    SchRoundRectangle,
    SchRoundRectangleRecord,
    SchRoundRectangleHandle
);
impl_handle_family!(
    SchEllipticalArc,
    SchEllipticalArcRecord,
    SchEllipticalArcHandle
);
impl_handle_family!(SchImage, SchImageRecord, SchImageHandle);
impl_handle_family!(SchDesignator, SchDesignatorRecord, SchDesignatorHandle);
impl_handle_family!(SchParameter, SchParameterRecord, SchParameterHandle);
impl_handle_family!(SchSymbol, SchSymbolRecord, SchSymbolHandle);
impl_handle_family!(SchLabel, SchLabelRecord, SchLabelHandle);
impl_handle_family!(SchPower, SchPowerRecord, SchPowerHandle);
impl_handle_family!(SchPort, SchPortRecord, SchPortHandle);
impl_handle_family!(SchNoERC, SchNoERCRecord, SchNoERCHandle);
impl_handle_family!(SchNetLabel, SchNetLabelRecord, SchNetLabelHandle);
impl_handle_family!(SchBus, SchBusRecord, SchBusHandle);
impl_handle_family!(SchWire, SchWireRecord, SchWireHandle);
impl_handle_family!(SchTextFrame, SchTextFrameRecord, SchTextFrameHandle);
impl_handle_family!(SchJunction, SchJunctionRecord, SchJunctionHandle);
impl_handle_family!(SchSheet, SchSheetRecord, SchSheetHandle);
impl_handle_family!(SchSheetName, SchSheetNameRecord, SchSheetNameHandle);
impl_handle_family!(
    SchSheetFileName,
    SchSheetFileNameRecord,
    SchSheetFileNameHandle
);
impl_handle_family!(SchBusEntry, SchBusEntryRecord, SchBusEntryHandle);
impl_handle_family!(SchSheetSymbol, SchSheetSymbolRecord, SchSheetSymbolHandle);
impl_handle_family!(SchSheetEntry, SchSheetEntryRecord, SchSheetEntryHandle);
impl_handle_family!(
    SchImplementationList,
    SchImplementationListRecord,
    SchImplementationListHandle
);
impl_handle_family!(
    SchImplementation,
    SchImplementationRecord,
    SchImplementationHandle
);
impl_handle_family!(
    SchMapDefinerList,
    SchMapDefinerListRecord,
    SchMapDefinerListHandle
);
impl_handle_family!(SchMapDefiner, SchMapDefinerRecord, SchMapDefinerHandle);
impl_handle_family!(
    SchImplementationParameters,
    SchImplementationParametersRecord,
    SchImplementationParametersHandle
);
impl_handle_family!(SchNote, SchNoteRecord, SchNoteHandle);
impl_handle_family!(SchBlanket, SchBlanketRecord, SchBlanketHandle);
impl_handle_family!(SchTaskHolder, SchTaskHolderRecord, SchTaskHolderHandle);

// ---------------------------------------------------------------------------
// PCB handle families
// ---------------------------------------------------------------------------

impl_handle_family!(PcbTrack, PcbTrackRecord, PcbTrackHandle);
impl_handle_family!(PcbArc, PcbArcRecord, PcbArcHandle);
impl_handle_family!(PcbConnection, PcbConnectionRecord, PcbConnectionHandle);
impl_handle_family!(PcbFill, PcbFillRecord, PcbFillHandle);
impl_handle_family!(PcbPad, PcbPadRecord, PcbPadHandle);
impl_handle_family!(PcbVia, PcbViaRecord, PcbViaHandle);
impl_handle_family!(PcbText, PcbTextRecord, PcbTextHandle);
impl_handle_family!(PcbRegion, PcbRegionRecord, PcbRegionHandle);
impl_handle_family!(
    PcbComponentBody,
    PcbComponentBodyRecord,
    PcbComponentBodyHandle
);
impl_handle_family!(
    PcbFootprintMetadata,
    PcbFootprintRecord,
    PcbFootprintMetadataHandle
);

// ---------------------------------------------------------------------------
// Group handles
// ---------------------------------------------------------------------------

/// Handle to a schematic component group (parent + children).
#[derive(Clone)]
pub struct SchComponentHandle {
    pub(crate) store: DocRef,
    pub(crate) group_id: GroupId,
}

impl SchComponentHandle {
    pub(crate) fn new(store: DocRef, group_id: GroupId) -> Self {
        Self { store, group_id }
    }

    pub fn group_id(&self) -> GroupId {
        self.group_id
    }

    /// Returns the stable semantic ID for this component group, if computed.
    pub fn semantic_id(&self) -> Option<crate::semantic_ids::SemanticId> {
        let mut store = self.store.borrow_mut();
        store.ensure_semantic_ids();
        store.group_semantic_id(self.group_id).cloned()
    }

    /// Read the parent component record.
    pub fn read(&self) -> SchComponentRecord {
        let store = self.store.borrow();
        let group = &store.groups[self.group_id];
        let node = &store.records[group.parent];
        SchComponentRecord::from_origin(node.origin.clone())
    }

    /// Write the parent component record.
    ///
    /// Also syncs the GroupMeta fields (lib_ref, description, part_count)
    /// so that save() produces a coherent FileHeader.
    pub fn write(&self, record: SchComponentRecord) -> crate::error::Result<()> {
        // Extract metadata fields before consuming the record.
        let new_lib_ref = record.lib_reference()?.to_string();
        let new_description = record.component_description()?.to_string();
        let new_part_count = record.part_count()? as i32;

        let mut store = self.store.borrow_mut();
        let group = &store.groups[self.group_id];
        let parent_id = group.parent;
        let node = &mut store.records[parent_id];
        node.origin = record.into_origin();
        node.mark_dirty();

        // Sync GroupMeta so save() reads current values.
        let group = &mut store.groups[self.group_id];
        if let crate::store::GroupMeta::SchComponent {
            ref mut lib_ref,
            ref mut description,
            ref mut part_count,
            ..
        } = group.meta
        {
            *lib_ref = new_lib_ref;
            *description = new_description;
            *part_count = new_part_count;
        }
        store.mark_semantic_ids_dirty();
        Ok(())
    }

    /// Get handles to all children of a given type.
    pub fn children<T: HandleFamily>(&self) -> crate::error::Result<Vec<T::Handle>> {
        let store = self.store.borrow();
        let group = &store.groups[self.group_id];
        group
            .children
            .iter()
            .filter(|&&id| {
                let rec = &store.records[id];
                rec.key == T::record_id() && rec.origin.is_binary() == T::is_binary()
            })
            .map(|&id| T::try_make_handle(self.store.clone(), id))
            .collect()
    }

    /// Count children of a given type.
    pub fn child_count<T: HandleFamily>(&self) -> usize {
        let store = self.store.borrow();
        let group = &store.groups[self.group_id];
        group
            .children
            .iter()
            .filter(|&&id| {
                let rec = &store.records[id];
                rec.key == T::record_id() && rec.origin.is_binary() == T::is_binary()
            })
            .count()
    }

    /// Total number of children (all types).
    pub fn children_len(&self) -> usize {
        let store = self.store.borrow();
        let group = &store.groups[self.group_id];
        group.children.len()
    }

    /// Get ALL children as (record_id_byte, RecordId) pairs.
    pub fn all_children(&self) -> Vec<(u8, RecordId)> {
        let store = self.store.borrow();
        let group = &store.groups[self.group_id];
        group
            .children
            .iter()
            .map(|&id| (store.records[id].key, id))
            .collect()
    }

    /// Library reference name (from group metadata).
    pub fn lib_ref(&self) -> String {
        let store = self.store.borrow();
        let group = &store.groups[self.group_id];
        match &group.meta {
            crate::store::GroupMeta::SchComponent { lib_ref, .. } => lib_ref.clone(),
            _ => String::new(),
        }
    }

    /// Component description (from group metadata).
    pub fn description(&self) -> String {
        let store = self.store.borrow();
        let group = &store.groups[self.group_id];
        match &group.meta {
            crate::store::GroupMeta::SchComponent { description, .. } => description.clone(),
            _ => String::new(),
        }
    }

    /// Part count (from group metadata).
    pub fn part_count(&self) -> i32 {
        let store = self.store.borrow();
        let group = &store.groups[self.group_id];
        match &group.meta {
            crate::store::GroupMeta::SchComponent { part_count, .. } => *part_count,
            _ => 1,
        }
    }

    /// Returns typed per-component sidecar streams (PinFrac, PinTextData, etc.).
    pub fn sidecar_streams(
        &self,
    ) -> crate::documents::schlib_streams::SchLibComponentSidecarStreamsMeta {
        let store = self.store.borrow();
        let group = &store.groups[self.group_id];
        match &group.meta {
            crate::store::GroupMeta::SchComponent {
                sidecar_streams, ..
            } => sidecar_streams.clone(),
            _ => crate::documents::schlib_streams::SchLibComponentSidecarStreamsMeta::default(),
        }
    }

    /// Replace typed per-component sidecar streams.
    pub fn set_sidecar_streams(
        &self,
        streams: crate::documents::schlib_streams::SchLibComponentSidecarStreamsMeta,
    ) {
        let mut store = self.store.borrow_mut();
        let group = &mut store.groups[self.group_id];
        if let crate::store::GroupMeta::SchComponent {
            sidecar_streams, ..
        } = &mut group.meta
        {
            *sidecar_streams = streams;
            store.mark_semantic_ids_dirty();
        }
    }

    /// Query children of a given type.
    pub fn query<T: HandleFamily>(&self, q: &str) -> crate::error::Result<T::Handle> {
        use crate::query::eval::evaluate;
        let parsed = crate::query::parse(q)?;

        let store = self.store.borrow();
        let group = &store.groups[self.group_id];
        let mut matches = Vec::new();
        for &id in &group.children {
            let rec = &store.records[id];
            if rec.key == T::record_id() && rec.origin.is_binary() == T::is_binary() {
                let all = std::slice::from_ref(rec);
                if !evaluate(&parsed, all).is_empty() {
                    matches.push(id);
                }
            }
        }

        match matches.len() {
            0 => Err(crate::error::AltiumError::NoMatch(q.to_string())),
            1 => T::try_make_handle(self.store.clone(), matches[0]),
            n => Err(crate::error::AltiumError::AmbiguousMatch(n, q.to_string())),
        }
    }

    /// Query all children of a given type.
    pub fn query_all<T: HandleFamily>(&self, q: &str) -> crate::error::Result<Vec<T::Handle>> {
        use crate::query::eval::evaluate;
        let parsed = crate::query::parse(q)?;

        let store = self.store.borrow();
        let group = &store.groups[self.group_id];
        let mut handles = Vec::new();
        for &id in &group.children {
            let rec = &store.records[id];
            if rec.key == T::record_id() && rec.origin.is_binary() == T::is_binary() {
                let all = std::slice::from_ref(rec);
                if !evaluate(&parsed, all).is_empty() {
                    handles.push(T::try_make_handle(self.store.clone(), id)?);
                }
            }
        }

        Ok(handles)
    }

    /// Insert a new record into the store and append it to this component's
    /// children list. Returns the new record's [`RecordId`].
    pub fn add_record(&self, mut node: crate::backing_store::RecordNode) -> RecordId {
        // Programmatically-added records must re-serialize from their current
        // origin, not from the stale template snapshot.
        node.mark_dirty();
        let mut store = self.store.borrow_mut();
        let record_id = store.insert_record(node);
        let group = &mut store.groups[self.group_id];
        group.children.push(record_id);
        // Maintain parallel index vector for SchDoc flattening.
        group.original_indices.push(usize::MAX);
        store.mark_semantic_ids_dirty();
        record_id
    }

    /// Insert a typed child record using high-level record APIs only.
    ///
    /// The record is converted into its backing origin internally and inserted
    /// as a new child of this component.
    pub fn add_child_record<R>(&self, record: R) -> RecordId
    where
        R: FromOrigin + RecordType,
    {
        let node = crate::backing_store::RecordNode::new(R::RECORD_ID, record.into_origin());
        self.add_record(node)
    }

    /// Construct a typed handle for a record in this component's store.
    pub fn handle_for<H: HandleFamily>(&self, rid: RecordId) -> crate::error::Result<H::Handle> {
        H::try_make_handle(self.store.clone(), rid)
    }

    /// Returns `true` if the given record has a binary origin.
    pub fn is_record_binary(&self, rid: RecordId) -> bool {
        let store = self.store.borrow();
        store.records[rid].origin.is_binary()
    }
}

/// Handle to a PCB footprint group (metadata + primitives).
#[derive(Clone)]
pub struct PcbFootprintHandle {
    pub(crate) store: DocRef,
    pub(crate) group_id: GroupId,
}

/// Raw footprint storage-level data that is not modeled by typed primitive
/// records but still participates in on-disk fidelity.
#[derive(Clone, Debug, Default)]
pub struct PcbFootprintStoragePassthrough {
    pub raw_pattern_name_block: Vec<u8>,
    pub raw_header: Vec<u8>,
    pub original_primitive_order: Vec<crate::backing_store::PcbPrimitiveRef>,
    pub sidecar_streams: crate::documents::pcblib_streams::PcbLibFootprintSidecarStreamsMeta,
}

impl PcbFootprintHandle {
    pub(crate) fn new(store: DocRef, group_id: GroupId) -> Self {
        Self { store, group_id }
    }

    pub fn group_id(&self) -> GroupId {
        self.group_id
    }

    /// Returns the stable semantic ID for this footprint group, if computed.
    pub fn semantic_id(&self) -> Option<crate::semantic_ids::SemanticId> {
        let mut store = self.store.borrow_mut();
        store.ensure_semantic_ids();
        store.group_semantic_id(self.group_id).cloned()
    }

    /// Read the footprint metadata record.
    pub fn read(&self) -> PcbFootprintRecord {
        let store = self.store.borrow();
        let group = &store.groups[self.group_id];
        let node = &store.records[group.parent];
        PcbFootprintRecord::from_origin(node.origin.clone())
    }

    /// Write the footprint metadata record.
    pub fn write(&self, record: PcbFootprintRecord) {
        let mut store = self.store.borrow_mut();
        let group = &store.groups[self.group_id];
        let parent_id = group.parent;
        let node = &mut store.records[parent_id];
        node.origin = record.into_origin();
        node.mark_dirty();
        store.mark_semantic_ids_dirty();
    }

    /// Get handles to all primitives of a given type.
    pub fn children<T: HandleFamily>(&self) -> crate::error::Result<Vec<T::Handle>> {
        let store = self.store.borrow();
        let group = &store.groups[self.group_id];
        group
            .children
            .iter()
            .filter(|&&id| {
                let rec = &store.records[id];
                rec.key == T::record_id() && rec.origin.is_binary() == T::is_binary()
            })
            .map(|&id| T::try_make_handle(self.store.clone(), id))
            .collect()
    }

    /// Count primitives of a given type.
    pub fn child_count<T: HandleFamily>(&self) -> usize {
        let store = self.store.borrow();
        let group = &store.groups[self.group_id];
        group
            .children
            .iter()
            .filter(|&&id| {
                let rec = &store.records[id];
                rec.key == T::record_id() && rec.origin.is_binary() == T::is_binary()
            })
            .count()
    }

    /// Total number of primitives (all types).
    pub fn children_len(&self) -> usize {
        let store = self.store.borrow();
        let group = &store.groups[self.group_id];
        group.children.len()
    }

    /// Get ALL primitives as (type_id_byte, RecordId) pairs.
    pub fn all_children(&self) -> Vec<(u8, RecordId)> {
        let store = self.store.borrow();
        let group = &store.groups[self.group_id];
        group
            .children
            .iter()
            .map(|&id| (store.records[id].key, id))
            .collect()
    }

    /// Footprint name (from group metadata).
    pub fn name(&self) -> String {
        let store = self.store.borrow();
        let group = &store.groups[self.group_id];
        match &group.meta {
            crate::store::GroupMeta::PcbFootprint { name, .. } => name.clone(),
            _ => String::new(),
        }
    }

    /// Returns storage-level passthrough bytes for this footprint.
    pub fn storage_passthrough(&self) -> PcbFootprintStoragePassthrough {
        let store = self.store.borrow();
        let group = &store.groups[self.group_id];
        match &group.meta {
            crate::store::GroupMeta::PcbFootprint {
                raw_pattern_name_block,
                raw_header,
                original_primitive_order,
                sidecar_streams,
                ..
            } => PcbFootprintStoragePassthrough {
                raw_pattern_name_block: raw_pattern_name_block.clone(),
                raw_header: raw_header.clone(),
                original_primitive_order: original_primitive_order.clone(),
                sidecar_streams: sidecar_streams.clone(),
            },
            _ => PcbFootprintStoragePassthrough::default(),
        }
    }

    /// Overwrite storage-level passthrough bytes for this footprint.
    pub fn set_storage_passthrough(&self, data: PcbFootprintStoragePassthrough) {
        let mut store = self.store.borrow_mut();
        let group = &mut store.groups[self.group_id];
        if let crate::store::GroupMeta::PcbFootprint {
            raw_pattern_name_block,
            raw_header,
            original_primitive_order,
            sidecar_streams,
            ..
        } = &mut group.meta
        {
            *raw_pattern_name_block = data.raw_pattern_name_block;
            *raw_header = data.raw_header;
            *original_primitive_order = data.original_primitive_order;
            *sidecar_streams = data.sidecar_streams;
        }
        store.mark_semantic_ids_dirty();
    }

    /// Query primitives of a given type.
    pub fn query<T: HandleFamily>(&self, q: &str) -> crate::error::Result<T::Handle> {
        use crate::query::eval::evaluate;
        let parsed = crate::query::parse(q)?;

        let store = self.store.borrow();
        let group = &store.groups[self.group_id];
        let mut matches = Vec::new();
        for &id in &group.children {
            let rec = &store.records[id];
            if rec.key == T::record_id() && rec.origin.is_binary() == T::is_binary() {
                let all = std::slice::from_ref(rec);
                if !evaluate(&parsed, all).is_empty() {
                    matches.push(id);
                }
            }
        }

        match matches.len() {
            0 => Err(crate::error::AltiumError::NoMatch(q.to_string())),
            1 => T::try_make_handle(self.store.clone(), matches[0]),
            n => Err(crate::error::AltiumError::AmbiguousMatch(n, q.to_string())),
        }
    }

    /// Query all primitives of a given type.
    pub fn query_all<T: HandleFamily>(&self, q: &str) -> crate::error::Result<Vec<T::Handle>> {
        use crate::query::eval::evaluate;
        let parsed = crate::query::parse(q)?;

        let store = self.store.borrow();
        let group = &store.groups[self.group_id];
        let mut handles = Vec::new();
        for &id in &group.children {
            let rec = &store.records[id];
            if rec.key == T::record_id() && rec.origin.is_binary() == T::is_binary() {
                let all = std::slice::from_ref(rec);
                if !evaluate(&parsed, all).is_empty() {
                    handles.push(T::try_make_handle(self.store.clone(), id)?);
                }
            }
        }

        Ok(handles)
    }

    /// Insert a new record into the store and append it to this footprint's
    /// children list. Also registers the record in `original_primitive_order`
    /// so it is included during serialization.
    ///
    /// Returns the new record's [`RecordId`].
    pub fn add_record(&self, mut node: crate::backing_store::RecordNode) -> RecordId {
        use crate::backing_store::PcbPrimitiveRef;
        use crate::store::GroupMeta;

        // Programmatically-added records must re-serialize from their current
        // origin, not from the stale template snapshot.
        node.mark_dirty();
        let mut store = self.store.borrow_mut();
        let type_id = node.key;
        let record_id = store.insert_record(node);

        let group = &mut store.groups[self.group_id];
        let child_index = group.children.len();
        group.children.push(record_id);
        group.original_indices.push(child_index);

        if let GroupMeta::PcbFootprint {
            original_primitive_order,
            ..
        } = &mut group.meta
        {
            original_primitive_order.push(PcbPrimitiveRef::new(type_id, child_index));
        }

        store.mark_semantic_ids_dirty();

        record_id
    }

    /// Insert a typed primitive record using high-level record APIs only.
    ///
    /// The record is converted into its backing origin internally and inserted
    /// as a new primitive of this footprint.
    pub fn add_primitive_record<R>(&self, record: R) -> RecordId
    where
        R: FromOrigin + RecordType,
    {
        let node = crate::backing_store::RecordNode::new(R::RECORD_ID, record.into_origin());
        self.add_record(node)
    }

    /// Construct a typed handle for a record in this footprint's store.
    pub fn handle_for<H: HandleFamily>(&self, rid: RecordId) -> crate::error::Result<H::Handle> {
        H::try_make_handle(self.store.clone(), rid)
    }
}

// ---------------------------------------------------------------------------
// HandleFamily for group handles (SchComponent, PcbFootprint)
// ---------------------------------------------------------------------------

pub enum SchComponent {}
impl HandleFamily for SchComponent {
    type Record = SchComponentRecord;
    type Handle = SchComponentHandle;

    fn try_make_handle(store: DocRef, id: RecordId) -> crate::error::Result<Self::Handle> {
        // The id passed here is the parent record id. Find which group owns it.
        let borrowed = store.borrow();
        for &gid in borrowed.group_ids() {
            let group = borrowed.group(gid);
            if group.parent == id {
                drop(borrowed);
                return Ok(SchComponentHandle::new(store, gid));
            }
        }
        Err(crate::error::AltiumError::InvalidRecord(format!(
            "SchComponent handle: no group owns record {:?}",
            id
        )))
    }
}

pub enum PcbFootprint {}
impl HandleFamily for PcbFootprint {
    type Record = PcbFootprintRecord;
    type Handle = PcbFootprintHandle;

    fn try_make_handle(store: DocRef, id: RecordId) -> crate::error::Result<Self::Handle> {
        // The id passed here is the parent record id. Find which group owns it.
        let borrowed = store.borrow();
        for &gid in borrowed.group_ids() {
            let group = borrowed.group(gid);
            if group.parent == id {
                drop(borrowed);
                return Ok(PcbFootprintHandle::new(store, gid));
            }
        }
        Err(crate::error::AltiumError::InvalidRecord(format!(
            "PcbFootprint handle: no group owns record {:?}",
            id
        )))
    }
}
