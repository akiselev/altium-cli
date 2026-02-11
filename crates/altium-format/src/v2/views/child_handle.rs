//! Typed handles for accessing child records within parent wrappers.
//!
//! These types provide type-safe, index-based access to children of a
//! parent view (e.g., pins within a component). The phantom type parameter
//! `T: WrapperFamily` ensures that only correctly-typed children are
//! accessed through a handle.

use std::marker::PhantomData;

use crate::v2::backing_store::RecordNode;
use crate::v2::traits::{LeafViewConstructor, WrapperFamily};

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

    /// Consume this handle, construct a typed view, pass it to the closure,
    /// and return the closure's result. The view is dropped (flushing any
    /// mutations) before this method returns.
    pub fn with_mut<R>(self, f: impl FnOnce(T::View<'_>) -> R) -> R
    where
        T: LeafViewConstructor,
    {
        let node = &mut self.children[self.index];
        let view = T::make_view(node);
        f(view)
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

    /// Consume this results set and call the closure for each matching child,
    /// providing a typed mutable view. Each view is dropped (flushing
    /// mutations) before the next iteration.
    pub fn for_each_mut(self, mut f: impl FnMut(T::View<'_>))
    where
        T: LeafViewConstructor,
    {
        let indices = self.indices;
        let children = self.children;
        for idx in indices {
            let node = &mut children[idx];
            let view = T::make_view(node);
            f(view);
        }
    }
}

// ---------------------------------------------------------------------------
// ChildrenMut — independent mutable access to children from split()
// ---------------------------------------------------------------------------

/// Mutable access to a parent's children slice, independent of the parent
/// record borrow.
///
/// Obtained via `SchComponentView::split()` or `PcbFootprintView::split()`.
/// Provides the same query/child_keys/with_child_mut methods as the parent
/// view's child section, enabling simultaneous parent+child borrowing.
pub struct ChildrenMut<'a> {
    pub(crate) children: &'a mut [RecordNode],
}

impl<'a> ChildrenMut<'a> {
    /// Query children for a single match of type `T`.
    pub fn query<T: WrapperFamily>(
        &mut self,
        q: &str,
    ) -> crate::error::Result<ChildHandle<'_, T>> {
        use crate::v2::query::eval::evaluate;
        let parsed = crate::v2::query::parse(q)?;

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
            _marker: PhantomData,
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

    /// Access a child by its `ChildKey`, constructing a typed view.
    pub fn with_child_mut<T: LeafViewConstructor, R>(
        &mut self,
        key: ChildKey<T>,
        f: impl FnOnce(T::View<'_>) -> R,
    ) -> R {
        let node = &mut self.children[key.index()];
        let view = T::make_view(node);
        f(view)
    }

    /// Returns the number of children.
    pub fn len(&self) -> usize {
        self.children.len()
    }

    /// Returns true if there are no children.
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }
}
