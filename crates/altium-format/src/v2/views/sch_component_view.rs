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
use crate::v2::traits::RecordType;

use super::child_ref::SchChildRef;
use super::leaf_wrappers::SchPinView;

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

    /// Number of child records.
    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    /// Count pins (record_id == 2).
    pub fn pin_count(&self) -> usize {
        self.children
            .iter()
            .filter(|c| c.key == SchPinRecord::RECORD_ID)
            .count()
    }

    /// Count children of a specific record type.
    pub fn count_by_type(&self, record_id: u8) -> usize {
        self.children.iter().filter(|c| c.key == record_id).count()
    }

    /// Iterate over all pins, providing owned (cloned) records for read access.
    pub fn for_each_pin(&self, mut f: impl FnMut(SchPinRecord)) {
        for child in self
            .children
            .iter()
            .filter(|c| c.key == SchPinRecord::RECORD_ID)
        {
            let rec = SchPinRecord::from_origin(child.origin.clone());
            f(rec);
        }
    }

    /// Iterate over all pins with mutable view access.
    ///
    /// Each `SchPinView` provides `Deref` access to `SchPinRecord` getters,
    /// and `DerefMut` access to setters. Changes are flushed on view drop.
    pub fn for_each_pin_mut(&mut self, mut f: impl FnMut(SchPinView<'_>)) {
        for child in self
            .children
            .iter_mut()
            .filter(|c| c.key == SchPinRecord::RECORD_ID)
        {
            let view = SchPinView::new(child);
            f(view);
        }
    }

    /// Iterate over ALL children with opaque read-only references.
    pub fn for_each_child(&self, mut f: impl FnMut(SchChildRef<'_>)) {
        for child in self.children.iter() {
            f(SchChildRef::new(child));
        }
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
