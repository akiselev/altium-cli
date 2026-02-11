//! Parent view wrapper for schematic components.
//!
//! [`SchComponentView`] provides lifetime-bounded access to a schematic
//! component record and all of its child records (pins, designators,
//! parameters, etc.) through a single borrowed view.

use crate::v2::backing_store::{RecordNode, RecordOrigin};
use crate::v2::records::SchComponentRecord;

/// A borrowed view over a schematic component and its child records.
///
/// This is a "parent wrapper" that provides access to both the component
/// record node and its owned children (pins, labels, designators, etc.).
/// The view borrows from a [`ComponentGroup`](crate::v2::backing_store::ComponentGroup)
/// via its `split_borrow()` method.
///
/// # Example
///
/// ```ignore
/// let (component, children) = group.split_borrow();
/// let view = SchComponentView::new(component, children);
/// println!("Component has {} pins", view.pin_count());
/// ```
pub struct SchComponentView<'a> {
    component: &'a mut RecordNode,
    children: &'a mut [RecordNode],
}

impl<'a> SchComponentView<'a> {
    /// Creates a new `SchComponentView` from a component record node and
    /// its children slice.
    pub fn new(component: &'a mut RecordNode, children: &'a mut [RecordNode]) -> Self {
        Self {
            component,
            children,
        }
    }

    /// Access the component record by cloning the origin.
    ///
    /// Returns a new `SchComponentRecord` constructed from a clone of the
    /// backing store origin. Changes to the returned record will NOT be
    /// reflected back -- use `record_origin_mut()` for modifications.
    pub fn record(&self) -> SchComponentRecord {
        SchComponentRecord::from_origin(self.component.origin.clone())
    }

    /// Access a mutable reference to the component record's origin for
    /// modifications.
    ///
    /// Marks the record node as dirty automatically.
    pub fn record_origin_mut(&mut self) -> &mut RecordOrigin {
        self.component.mark_dirty();
        &mut self.component.origin
    }

    /// Number of child records.
    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    /// Count pins (record_id == 2).
    pub fn pin_count(&self) -> usize {
        self.children.iter().filter(|c| c.key == 2).count()
    }

    /// Split borrow: component record node + children slice.
    ///
    /// This enables simultaneous mutable access to the component and its
    /// children, which is safe because they are stored in separate memory.
    pub fn split(&mut self) -> (&mut RecordNode, &mut [RecordNode]) {
        (self.component, self.children)
    }

    /// Iterate over children matching a record ID.
    pub fn children_by_type(&self, record_id: u8) -> impl Iterator<Item = &RecordNode> {
        self.children.iter().filter(move |c| c.key == record_id)
    }

    /// Iterate mutably over children matching a record ID.
    pub fn children_by_type_mut(
        &mut self,
        record_id: u8,
    ) -> impl Iterator<Item = &mut RecordNode> {
        self.children
            .iter_mut()
            .filter(move |c| c.key == record_id)
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
