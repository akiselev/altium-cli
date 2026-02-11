//! PCB footprint view type.
//!
//! Provides ergonomic access to a PCB footprint's metadata and primitives
//! through `Deref<Target = PcbFootprintRecord>` and child navigation methods.

use crate::v2::backing_store::{RecordNode, RecordOrigin};
use crate::v2::records::{PcbFootprintRecord, PcbPadRecord};
use crate::v2::traits::{LeafViewConstructor, RecordType, WrapperFamily};

use super::child_handle::{ChildHandle, ChildKey, ChildResults, ChildrenMut};

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

    /// Query primitives for a single match of type `T`.
    pub fn query<T: WrapperFamily>(
        &mut self,
        q: &str,
    ) -> crate::error::Result<ChildHandle<'_, T>> {
        use crate::v2::query::eval::evaluate;
        let parsed = crate::v2::query::parse(q)?;

        let matching: Vec<usize> = self
            .primitives
            .iter()
            .enumerate()
            .filter(|(_, node)| node.key == T::record_id())
            .filter(|(_, node)| {
                let all = std::slice::from_ref(*node);
                !evaluate(&parsed, all).is_empty()
            })
            .map(|(i, _)| i)
            .collect();

        match matching.len() {
            0 => Err(crate::error::AltiumError::NoMatch(q.to_string())),
            1 => Ok(ChildHandle::new(&mut *self.primitives, matching[0])),
            n => Err(crate::error::AltiumError::AmbiguousMatch(n, q.to_string())),
        }
    }

    /// Query primitives for all matches of type `T`.
    pub fn query_all<T: WrapperFamily>(
        &mut self,
        q: &str,
    ) -> crate::error::Result<ChildResults<'_, T>> {
        use crate::v2::query::eval::evaluate;
        let parsed = crate::v2::query::parse(q)?;

        let indices: Vec<usize> = self
            .primitives
            .iter()
            .enumerate()
            .filter(|(_, node)| node.key == T::record_id())
            .filter(|(_, node)| {
                let all = std::slice::from_ref(*node);
                !evaluate(&parsed, all).is_empty()
            })
            .map(|(i, _)| i)
            .collect();

        Ok(ChildResults {
            children: &mut *self.primitives,
            indices,
            _marker: std::marker::PhantomData,
        })
    }

    /// Returns an iterator over `ChildKey<T>` for all primitives of type `T`.
    pub fn child_keys<T: WrapperFamily>(&self) -> impl Iterator<Item = ChildKey<T>> + use<'_, T> {
        self.primitives
            .iter()
            .enumerate()
            .filter(|(_, node)| node.key == T::record_id())
            .map(|(i, _)| ChildKey::new(i))
    }

    /// Access a primitive by its `ChildKey`, constructing a typed view.
    pub fn with_child_mut<T: LeafViewConstructor, R>(
        &mut self,
        key: ChildKey<T>,
        f: impl FnOnce(T::View<'_>) -> R,
    ) -> R {
        let node = &mut self.primitives[key.index()];
        let view = T::make_view(node);
        f(view)
    }

    /// Split the view into independent metadata record and primitives access.
    pub fn split(&mut self) -> (&mut PcbFootprintRecord, ChildrenMut<'_>) {
        self.dirty = true;
        (
            &mut self.cached_metadata,
            ChildrenMut {
                children: &mut *self.primitives,
            },
        )
    }

    /// Returns the total number of primitives (all types).
    pub fn primitives_len(&self) -> usize {
        self.primitives.len()
    }

    /// Returns an iterator over the type IDs of all primitives.
    pub fn primitive_type_ids(&self) -> impl Iterator<Item = u8> + '_ {
        self.primitives.iter().map(|n| n.key)
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

// ---------------------------------------------------------------------------
// WrapperFamily for PcbFootprint (parent wrapper)
// ---------------------------------------------------------------------------

/// Marker type for the PCB footprint `WrapperFamily`.
///
/// Used as a type parameter in query APIs:
/// ```ignore
/// doc.query::<PcbFootprint>("SOIC-8")?
/// ```
pub enum PcbFootprint {}

impl crate::v2::traits::WrapperFamily for PcbFootprint {
    type Record = PcbFootprintRecord;
    type View<'a> = PcbFootprintView<'a>;
}
