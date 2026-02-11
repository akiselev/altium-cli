//! PCB footprint view type.
//!
//! Provides ergonomic access to a PCB footprint's metadata and primitives
//! through `Deref<Target = PcbFootprintRecord>` and child navigation methods.

use crate::v2::backing_store::{RecordNode, RecordOrigin};
use crate::v2::records::{PcbArcRecord, PcbFootprintRecord, PcbPadRecord, PcbTrackRecord};
use crate::v2::traits::RecordType;

use super::child_ref::PcbChildRef;
use super::leaf_wrappers::{PcbArcView, PcbPadView, PcbTrackView};

/// View over a PCB footprint's metadata and primitive records.
///
/// Provides `Deref<Target = PcbFootprintRecord>` for reading metadata
/// fields (pattern, description, height, etc.) and `DerefMut` for writing.
/// On drop, if modified, the cached metadata is flushed back to the node.
pub struct PcbFootprintView<'a> {
    metadata_node: &'a mut RecordNode,
    cached_metadata: PcbFootprintRecord,
    dirty: bool,
    primitives: &'a mut Vec<RecordNode>,
}

impl<'a> PcbFootprintView<'a> {
    /// Creates a new footprint view from the metadata node and primitives vector.
    pub fn new(
        metadata_node: &'a mut RecordNode,
        primitives: &'a mut Vec<RecordNode>,
    ) -> Self {
        let cached_metadata = PcbFootprintRecord::from_origin(metadata_node.origin.clone());
        Self {
            metadata_node,
            cached_metadata,
            dirty: false,
            primitives,
        }
    }

    /// Returns the number of primitives in this footprint.
    pub fn primitive_count(&self) -> usize {
        self.primitives.len()
    }

    /// Count pads in this footprint.
    pub fn pad_count(&self) -> usize {
        self.primitives
            .iter()
            .filter(|p| p.key == PcbPadRecord::RECORD_ID)
            .count()
    }

    /// Count tracks in this footprint.
    pub fn track_count(&self) -> usize {
        self.primitives
            .iter()
            .filter(|p| p.key == PcbTrackRecord::RECORD_ID)
            .count()
    }

    /// Count arcs in this footprint.
    pub fn arc_count(&self) -> usize {
        self.primitives
            .iter()
            .filter(|p| p.key == PcbArcRecord::RECORD_ID)
            .count()
    }

    /// Count primitives of a specific type.
    pub fn count_by_type(&self, type_id: u8) -> usize {
        self.primitives
            .iter()
            .filter(|p| p.key == type_id)
            .count()
    }

    /// Iterate over all pads, providing owned (cloned) records for read access.
    pub fn for_each_pad(&self, mut f: impl FnMut(PcbPadRecord)) {
        for prim in self
            .primitives
            .iter()
            .filter(|p| p.key == PcbPadRecord::RECORD_ID)
        {
            let rec = PcbPadRecord::from_origin(prim.origin.clone());
            f(rec);
        }
    }

    /// Iterate over all pads with mutable view access.
    pub fn for_each_pad_mut(&mut self, mut f: impl FnMut(PcbPadView<'_>)) {
        for prim in self
            .primitives
            .iter_mut()
            .filter(|p| p.key == PcbPadRecord::RECORD_ID)
        {
            let view = PcbPadView::new(prim);
            f(view);
        }
    }

    /// Iterate over all tracks, providing owned (cloned) records for read access.
    pub fn for_each_track(&self, mut f: impl FnMut(PcbTrackRecord)) {
        for prim in self
            .primitives
            .iter()
            .filter(|p| p.key == PcbTrackRecord::RECORD_ID)
        {
            let rec = PcbTrackRecord::from_origin(prim.origin.clone());
            f(rec);
        }
    }

    /// Iterate over all tracks with mutable view access.
    pub fn for_each_track_mut(&mut self, mut f: impl FnMut(PcbTrackView<'_>)) {
        for prim in self
            .primitives
            .iter_mut()
            .filter(|p| p.key == PcbTrackRecord::RECORD_ID)
        {
            let view = PcbTrackView::new(prim);
            f(view);
        }
    }

    /// Iterate over all arcs with mutable view access.
    pub fn for_each_arc_mut(&mut self, mut f: impl FnMut(PcbArcView<'_>)) {
        for prim in self
            .primitives
            .iter_mut()
            .filter(|p| p.key == PcbArcRecord::RECORD_ID)
        {
            let view = PcbArcView::new(prim);
            f(view);
        }
    }

    /// Iterate over ALL primitives with opaque read-only references.
    pub fn for_each_primitive(&self, mut f: impl FnMut(PcbChildRef<'_>)) {
        for prim in self.primitives.iter() {
            f(PcbChildRef::new(prim));
        }
    }

    /// Add a new pad using a template origin and a configure closure.
    pub fn add_pad(
        &mut self,
        template: fn() -> RecordOrigin,
        f: impl FnOnce(&mut PcbPadRecord),
    ) {
        let origin = template();
        let mut rec = PcbPadRecord::from_origin(origin);
        f(&mut rec);
        let node = RecordNode::new(PcbPadRecord::RECORD_ID, rec.origin().clone());
        self.primitives.push(node);
    }
}

impl<'a> std::ops::Deref for PcbFootprintView<'a> {
    type Target = PcbFootprintRecord;
    fn deref(&self) -> &PcbFootprintRecord {
        &self.cached_metadata
    }
}

impl<'a> std::ops::DerefMut for PcbFootprintView<'a> {
    fn deref_mut(&mut self) -> &mut PcbFootprintRecord {
        self.dirty = true;
        &mut self.cached_metadata
    }
}

impl<'a> Drop for PcbFootprintView<'a> {
    fn drop(&mut self) {
        if self.dirty {
            self.metadata_node.origin = self.cached_metadata.origin().clone();
            self.metadata_node.mark_dirty();
        }
    }
}
