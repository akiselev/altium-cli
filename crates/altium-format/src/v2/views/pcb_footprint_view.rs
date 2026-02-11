//! PCB footprint view type.
//!
//! Provides ergonomic access to a PCB footprint and its primitives
//! through a lifetime-bounded view over the backing store.

use crate::v2::backing_store::RecordNode;

/// View over a PCB footprint's metadata and primitive records.
///
/// Similar to [`SchComponentView`], this provides split-borrow access
/// to both the metadata record and its child primitive records.
pub struct PcbFootprintView<'a> {
    /// The footprint metadata record.
    pub metadata: &'a mut RecordNode,
    /// The primitive records (tracks, pads, arcs, etc.).
    pub primitives: &'a mut [RecordNode],
}

impl<'a> PcbFootprintView<'a> {
    /// Creates a new footprint view from the metadata and primitives.
    pub fn new(metadata: &'a mut RecordNode, primitives: &'a mut [RecordNode]) -> Self {
        Self {
            metadata,
            primitives,
        }
    }

    /// Returns the number of primitives in this footprint.
    pub fn primitive_count(&self) -> usize {
        self.primitives.len()
    }

    /// Returns split borrows: the metadata and its primitives simultaneously.
    pub fn split(&mut self) -> (&mut RecordNode, &mut [RecordNode]) {
        (self.metadata, self.primitives)
    }
}
