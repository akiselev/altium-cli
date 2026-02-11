//! Typed handles for accessing child records within parent wrappers.
//!
//! These types provide type-safe, index-based access to children of a
//! parent view (e.g., pins within a component). The phantom type parameter
//! `T: WrapperFamily` ensures that only correctly-typed children are
//! accessed through a handle.

use std::marker::PhantomData;

use crate::v2::backing_store::RecordNode;
use crate::v2::traits::WrapperFamily;

// ---------------------------------------------------------------------------
// ChildKey
// ---------------------------------------------------------------------------

/// A typed index into a children slice, parameterized by the wrapper family.
///
/// `ChildKey` is a lightweight handle that identifies a specific child
/// record by its index. It carries the `WrapperFamily` type parameter to
/// ensure type safety when resolving the key back to a concrete view.
pub struct ChildKey<T: WrapperFamily> {
    pub(crate) index: usize,
    pub(crate) _marker: PhantomData<T>,
}

impl<T: WrapperFamily> ChildKey<T> {
    /// Creates a new `ChildKey` for the given index.
    pub fn new(index: usize) -> Self {
        Self {
            index,
            _marker: PhantomData,
        }
    }

    /// Returns the index this key points to.
    pub fn index(&self) -> usize {
        self.index
    }
}

// Manual Clone/Copy since PhantomData<T> doesn't require T: Clone.
impl<T: WrapperFamily> Clone for ChildKey<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: WrapperFamily> Copy for ChildKey<T> {}

impl<T: WrapperFamily> std::fmt::Debug for ChildKey<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChildKey")
            .field("index", &self.index)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// ChildHandle
// ---------------------------------------------------------------------------

/// A mutable handle to a specific child record within a children slice.
///
/// `ChildHandle` borrows the entire children slice and provides access to
/// a single record at the given index. The `WrapperFamily` type parameter
/// indicates the expected record type.
pub struct ChildHandle<'a, T: WrapperFamily> {
    pub(crate) children: &'a mut [RecordNode],
    pub(crate) index: usize,
    pub(crate) _marker: PhantomData<T>,
}

impl<'a, T: WrapperFamily> ChildHandle<'a, T> {
    /// Creates a new `ChildHandle` for the given index within the children
    /// slice.
    pub fn new(children: &'a mut [RecordNode], index: usize) -> Self {
        Self {
            children,
            index,
            _marker: PhantomData,
        }
    }

    /// Returns the index this handle points to.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Returns a shared reference to the record node at this handle's index.
    pub fn node(&self) -> &RecordNode {
        &self.children[self.index]
    }

    /// Returns a mutable reference to the record node at this handle's index.
    pub fn node_mut(&mut self) -> &mut RecordNode {
        &mut self.children[self.index]
    }
}

// ---------------------------------------------------------------------------
// ChildResults
// ---------------------------------------------------------------------------

/// A collection of matching child record indices within a children slice.
///
/// `ChildResults` stores a set of indices into the children slice that
/// matched some filter criterion (e.g., all children with a specific
/// record type). It provides length/emptiness queries and can be iterated
/// to produce individual `ChildKey` values.
pub struct ChildResults<'a, T: WrapperFamily> {
    pub(crate) children: &'a mut [RecordNode],
    pub(crate) indices: Vec<usize>,
    pub(crate) _marker: PhantomData<T>,
}

impl<'a, T: WrapperFamily> ChildResults<'a, T> {
    /// Returns the number of matching children.
    pub fn len(&self) -> usize {
        self.indices.len()
    }

    /// Returns true if no children matched.
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// Returns the matching indices.
    pub fn indices(&self) -> &[usize] {
        &self.indices
    }

    /// Converts the results into a vector of `ChildKey` values.
    pub fn keys(&self) -> Vec<ChildKey<T>> {
        self.indices.iter().map(|&i| ChildKey::new(i)).collect()
    }
}
