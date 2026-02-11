//! Parent view wrapper for schematic components.
//!
//! [`SchComponentView`] provides lifetime-bounded access to a schematic
//! component record and all of its child records (pins, designators,
//! parameters, etc.) through a single borrowed view.
//!
//! The view caches the component record for efficient field access through
//! `Deref<Target = SchComponentRecord>`. Mutations through `DerefMut` set
//! a dirty flag, and on `Drop` the cached record's origin is flushed back
//! to the underlying `RecordNode`.

use crate::v2::backing_store::{RecordNode, RecordOrigin};
use crate::v2::records::{SchComponentRecord, SchPinRecord};
use crate::v2::traits::{LeafViewConstructor, RecordType, WrapperFamily};

use super::child_handle::{ChildHandle, ChildKey, ChildResults, ChildrenMut};

/// A borrowed view over a schematic component and its child records.
///
/// Provides `Deref<Target = SchComponentRecord>` for reading component
/// fields and `DerefMut` for writing them. On drop, if modified, the
/// cached record's origin is flushed back to the backing store node.
///
/// # Example
///
/// ```ignore
/// let (component, children) = group.split_borrow();
/// let mut view = SchComponentView::new(component, children);
/// println!("Ref: {}", view.lib_reference());
/// view.set_lib_reference(LibReference::from("NewRef"));
/// // On drop, changes are flushed to the node.
/// ```
pub struct SchComponentView<'a> {
    node: &'a mut RecordNode,
    cached: SchComponentRecord,
    dirty: bool,
    children: &'a mut Vec<RecordNode>,
}

impl<'a> SchComponentView<'a> {
    /// Creates a new `SchComponentView` from a component record node and
    /// its children vector.
    pub fn new(node: &'a mut RecordNode, children: &'a mut Vec<RecordNode>) -> Self {
        let cached = SchComponentRecord::from_origin(node.origin.clone());
        Self {
            node,
            cached,
            dirty: false,
            children,
        }
    }

    /// Query children for a single match of type `T`.
    ///
    /// Parses the AQL query string, evaluates it against children of type `T`,
    /// and returns a `ChildHandle` if exactly one match is found.
    /// Returns `NoMatch` if none match, or `AmbiguousMatch` if more than one matches.
    pub fn query<T: WrapperFamily>(
        &mut self,
        q: &str,
    ) -> crate::error::Result<ChildHandle<'_, T>> {
        use crate::v2::query::eval::evaluate;
        let parsed = crate::v2::query::parse(q)?;

        // Collect indices of children matching both the record type and query
        let matching: Vec<usize> = self
            .children
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
            1 => Ok(ChildHandle::new(&mut *self.children, matching[0])),
            n => Err(crate::error::AltiumError::AmbiguousMatch(n, q.to_string())),
        }
    }

    /// Query children for all matches of type `T`.
    ///
    /// Returns a `ChildResults` containing all matching child indices.
    pub fn query_all<T: WrapperFamily>(
        &mut self,
        q: &str,
    ) -> crate::error::Result<ChildResults<'_, T>> {
        use crate::v2::query::eval::evaluate;
        let parsed = crate::v2::query::parse(q)?;

        let indices: Vec<usize> = self
            .children
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
            children: &mut *self.children,
            indices,
            _marker: std::marker::PhantomData,
        })
    }

    /// Returns an iterator over `ChildKey<T>` for all children of type `T`.
    pub fn child_keys<T: WrapperFamily>(&self) -> impl Iterator<Item = ChildKey<T>> + use<'_, T> {
        self.children
            .iter()
            .enumerate()
            .filter(|(_, node)| node.key == T::record_id())
            .map(|(i, _)| ChildKey::new(i))
    }

    /// Access a child by its `ChildKey`, constructing a typed view and
    /// passing it to the closure. The view is dropped (flushing mutations)
    /// before this method returns.
    pub fn with_child_mut<T: LeafViewConstructor, R>(
        &mut self,
        key: ChildKey<T>,
        f: impl FnOnce(T::View<'_>) -> R,
    ) -> R {
        let node = &mut self.children[key.index()];
        let view = T::make_view(node);
        f(view)
    }

    /// Split the view into independent parent record and children access.
    ///
    /// This allows simultaneous reading/writing of the parent component
    /// record and querying/mutating children without borrow conflicts.
    /// The parent is returned as `&mut SchComponentRecord` (sets dirty flag).
    pub fn split(&mut self) -> (&mut SchComponentRecord, ChildrenMut<'_>) {
        self.dirty = true;
        (
            &mut self.cached,
            ChildrenMut {
                children: &mut *self.children,
            },
        )
    }

    /// Returns the total number of children (all types).
    pub fn children_len(&self) -> usize {
        self.children.len()
    }

    /// Returns an iterator over the record IDs of all children.
    pub fn child_record_ids(&self) -> impl Iterator<Item = u8> + '_ {
        self.children.iter().map(|n| n.key)
    }

    /// Add a new pin using a template origin and a configure closure.
    pub fn add_pin(
        &mut self,
        template: fn() -> RecordOrigin,
        f: impl FnOnce(&mut SchPinRecord),
    ) {
        let origin = template();
        let mut rec = SchPinRecord::from_origin(origin);
        f(&mut rec);
        let node = RecordNode::new(SchPinRecord::RECORD_ID, rec.origin().clone());
        self.children.push(node);
    }
}

impl<'a> std::ops::Deref for SchComponentView<'a> {
    type Target = SchComponentRecord;
    fn deref(&self) -> &SchComponentRecord {
        &self.cached
    }
}

impl<'a> std::ops::DerefMut for SchComponentView<'a> {
    fn deref_mut(&mut self) -> &mut SchComponentRecord {
        self.dirty = true;
        &mut self.cached
    }
}

impl<'a> Drop for SchComponentView<'a> {
    fn drop(&mut self) {
        if self.dirty {
            self.node.origin = self.cached.origin().clone();
            self.node.mark_dirty();
        }
    }
}

// ---------------------------------------------------------------------------
// WrapperFamily for SchComponent (parent wrapper)
// ---------------------------------------------------------------------------

/// Marker type for the schematic component `WrapperFamily`.
///
/// Used as a type parameter in query APIs:
/// ```ignore
/// doc.query::<SchComponent>("DESIGNATOR == 'U1'")?
/// ```
pub enum SchComponent {}

impl crate::v2::traits::WrapperFamily for SchComponent {
    type Record = SchComponentRecord;
    type View<'a> = SchComponentView<'a>;
}
