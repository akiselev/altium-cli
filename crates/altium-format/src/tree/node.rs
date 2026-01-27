//! Node identification types for tree structures.

use std::fmt;

/// Unique identifier for a record within a tree.
///
/// RecordId is a lightweight handle that refers to a record by its index
/// in the tree's internal storage. It is only valid within the context
/// of the tree it was created from.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct RecordId(u32);

impl RecordId {
    /// The root record ID (index 0).
    pub const ROOT: RecordId = RecordId(0);

    /// Create a new RecordId from an index.
    #[inline]
    pub const fn new(index: u32) -> Self {
        RecordId(index)
    }

    /// Get the underlying index.
    #[inline]
    pub const fn index(self) -> u32 {
        self.0
    }

    /// Create a RecordId from a usize index.
    #[inline]
    pub fn from_usize(index: usize) -> Self {
        RecordId(index as u32)
    }

    /// Get the index as usize.
    #[inline]
    pub fn as_usize(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Debug for RecordId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RecordId({})", self.0)
    }
}

impl fmt::Display for RecordId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl From<u32> for RecordId {
    fn from(index: u32) -> Self {
        RecordId(index)
    }
}

impl From<usize> for RecordId {
    fn from(index: usize) -> Self {
        RecordId(index as u32)
    }
}

impl From<RecordId> for u32 {
    fn from(id: RecordId) -> Self {
        id.0
    }
}

impl From<RecordId> for usize {
    fn from(id: RecordId) -> Self {
        id.0 as usize
    }
}

/// Reference to a parent record.
///
/// This enum provides a type-safe way to represent parent references,
/// distinguishing between root records (no parent) and records with
/// a valid parent reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ParentRef {
    /// This record is a root (no parent).
    #[default]
    Root,
    /// This record has a parent at the given index.
    Index(RecordId),
}

impl ParentRef {
    /// Create a ParentRef from an owner index value.
    ///
    /// Negative values or values that would be invalid indexes
    /// are treated as Root.
    pub fn from_owner_index(index: i32) -> Self {
        if index < 0 {
            ParentRef::Root
        } else {
            ParentRef::Index(RecordId::new(index as u32))
        }
    }

    /// Convert to an owner index value.
    ///
    /// Root returns -1, otherwise returns the index.
    pub fn to_owner_index(self) -> i32 {
        match self {
            ParentRef::Root => -1,
            ParentRef::Index(id) => id.0 as i32,
        }
    }

    /// Check if this is a root reference.
    pub fn is_root(self) -> bool {
        matches!(self, ParentRef::Root)
    }

    /// Get the parent RecordId, if any.
    pub fn id(self) -> Option<RecordId> {
        match self {
            ParentRef::Root => None,
            ParentRef::Index(id) => Some(id),
        }
    }
}

impl From<i32> for ParentRef {
    fn from(index: i32) -> Self {
        ParentRef::from_owner_index(index)
    }
}

impl From<Option<RecordId>> for ParentRef {
    fn from(id: Option<RecordId>) -> Self {
        match id {
            Some(id) => ParentRef::Index(id),
            None => ParentRef::Root,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_id() {
        let id = RecordId::new(42);
        assert_eq!(id.index(), 42);
        assert_eq!(format!("{}", id), "#42");
        assert_eq!(format!("{:?}", id), "RecordId(42)");
    }

    #[test]
    fn test_parent_ref() {
        assert!(ParentRef::Root.is_root());
        assert!(!ParentRef::Index(RecordId::new(0)).is_root());

        assert_eq!(ParentRef::from_owner_index(-1), ParentRef::Root);
        assert_eq!(
            ParentRef::from_owner_index(5),
            ParentRef::Index(RecordId::new(5))
        );

        assert_eq!(ParentRef::Root.to_owner_index(), -1);
        assert_eq!(ParentRef::Index(RecordId::new(5)).to_owner_index(), 5);
    }
}
