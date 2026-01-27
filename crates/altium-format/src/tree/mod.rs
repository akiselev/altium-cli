//! Tree structure support for Altium record hierarchies.
//!
//! Altium records form tree structures where parent-child relationships are
//! encoded via the OWNERINDEX parameter. This module provides:
//!
//! - [`RecordTree`] - Generic tree structure with navigation methods
//! - [`RecordId`] - Unique identifier for records within a tree
//! - [`TreeWalker`] - Depth-first iterator over tree nodes
//!
//! # Example
//!
//! ```ignore
//! use altium_format::tree::RecordTree;
//! use altium_format::records::sch::SchRecord;
//!
//! // Build tree from flat record list
//! let tree = RecordTree::from_records(records);
//!
//! // Navigate the tree
//! for (id, record) in tree.roots() {
//!     println!("Root: {:?}", record);
//!     for (child_id, child) in tree.children(id) {
//!         println!("  Child: {:?}", child);
//!     }
//! }
//!
//! // Walk entire tree depth-first
//! for (id, record, depth) in tree.walk_depth_first() {
//!     println!("{:indent$}{:?}", "", record, indent = depth * 2);
//! }
//! ```

mod node;
mod walker;

pub use node::{ParentRef, RecordId};
pub use walker::{BreadthFirstWalker, TreeWalker};

use std::collections::HashMap;

/// A trait for records that can participate in a tree structure.
///
/// Records must provide their owner index (parent reference) to build
/// the tree relationships.
pub trait TreeRecord {
    /// Get the owner index (parent reference).
    /// Returns the index of the parent record, or a negative value for root records.
    fn owner_index(&self) -> i32;

    /// Set the owner index.
    fn set_owner_index(&mut self, index: i32);
}

/// Generic tree structure for Altium records.
///
/// Provides efficient navigation of parent-child relationships that are
/// encoded via OWNERINDEX in the original flat record list.
#[derive(Debug, Clone)]
pub struct RecordTree<R> {
    /// All records in the tree (indexed by RecordId).
    records: Vec<R>,
    /// Parent lookup: child_id -> parent_id
    parents: HashMap<RecordId, RecordId>,
    /// Children lookup: parent_id -> [child_ids]
    children: HashMap<RecordId, Vec<RecordId>>,
    /// Root records (no parent or invalid OWNERINDEX).
    roots: Vec<RecordId>,
}

impl<R: TreeRecord> RecordTree<R> {
    /// Build a tree from a flat list of records using OWNERINDEX relationships.
    ///
    /// Records with OWNERINDEX < 0 or >= len are treated as roots.
    pub fn from_records(records: Vec<R>) -> Self {
        let mut tree = RecordTree {
            records,
            parents: HashMap::new(),
            children: HashMap::new(),
            roots: Vec::new(),
        };
        tree.rebuild_relationships();
        tree
    }

    /// Rebuild parent-child relationships from OWNERINDEX values.
    fn rebuild_relationships(&mut self) {
        self.parents.clear();
        self.children.clear();
        self.roots.clear();

        let len = self.records.len();

        for (idx, record) in self.records.iter().enumerate() {
            let child_id = RecordId::new(idx as u32);
            let owner_index = record.owner_index();

            // Check if this is a root (invalid owner index)
            if owner_index < 0 || (owner_index as usize) >= len {
                self.roots.push(child_id);
            } else {
                let parent_id = RecordId::new(owner_index as u32);
                self.parents.insert(child_id, parent_id);
                self.children.entry(parent_id).or_default().push(child_id);
            }
        }
    }

    /// Get the number of records in the tree.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Check if the tree is empty.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Get a record by ID.
    pub fn get(&self, id: RecordId) -> Option<&R> {
        self.records.get(id.index() as usize)
    }

    /// Get a mutable record by ID.
    pub fn get_mut(&mut self, id: RecordId) -> Option<&mut R> {
        self.records.get_mut(id.index() as usize)
    }

    /// Get the record at a specific index.
    pub fn get_by_index(&self, index: usize) -> Option<&R> {
        self.records.get(index)
    }

    /// Iterate over all records with their IDs.
    pub fn iter(&self) -> impl Iterator<Item = (RecordId, &R)> {
        self.records
            .iter()
            .enumerate()
            .map(|(i, r)| (RecordId::new(i as u32), r))
    }

    /// Iterate over all records mutably with their IDs.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (RecordId, &mut R)> {
        self.records
            .iter_mut()
            .enumerate()
            .map(|(i, r)| (RecordId::new(i as u32), r))
    }

    /// Get root records (records with no valid parent).
    pub fn roots(&self) -> impl Iterator<Item = (RecordId, &R)> {
        self.roots
            .iter()
            .filter_map(move |&id| self.records.get(id.index() as usize).map(|r| (id, r)))
    }

    /// Get the number of root records.
    pub fn root_count(&self) -> usize {
        self.roots.len()
    }

    /// Check if a record is a root.
    pub fn is_root(&self, id: RecordId) -> bool {
        self.roots.contains(&id)
    }

    /// Get the parent of a record.
    pub fn parent(&self, id: RecordId) -> Option<(RecordId, &R)> {
        self.parents.get(&id).and_then(|&parent_id| {
            self.records
                .get(parent_id.index() as usize)
                .map(|r| (parent_id, r))
        })
    }

    /// Get the parent ID of a record (without the record itself).
    pub fn parent_id(&self, id: RecordId) -> Option<RecordId> {
        self.parents.get(&id).copied()
    }

    /// Get children of a record.
    pub fn children(&self, id: RecordId) -> impl Iterator<Item = (RecordId, &R)> {
        self.children
            .get(&id)
            .map(|ids| ids.as_slice())
            .unwrap_or(&[])
            .iter()
            .filter_map(move |&child_id| {
                self.records
                    .get(child_id.index() as usize)
                    .map(|r| (child_id, r))
            })
    }

    /// Get the number of children for a record.
    pub fn child_count(&self, id: RecordId) -> usize {
        self.children.get(&id).map(|c| c.len()).unwrap_or(0)
    }

    /// Check if a record has children.
    pub fn has_children(&self, id: RecordId) -> bool {
        self.children
            .get(&id)
            .map(|c| !c.is_empty())
            .unwrap_or(false)
    }

    /// Get all ancestors of a record (parent, grandparent, etc.).
    pub fn ancestors(&self, id: RecordId) -> impl Iterator<Item = (RecordId, &R)> {
        AncestorIterator {
            tree: self,
            current: self.parent_id(id),
        }
    }

    /// Get all descendants of a record (children, grandchildren, etc.).
    pub fn descendants(&self, id: RecordId) -> impl Iterator<Item = (RecordId, &R)> {
        DescendantIterator {
            tree: self,
            stack: self.children.get(&id).cloned().unwrap_or_default(),
        }
    }

    /// Get the depth of a record (0 for roots).
    pub fn depth(&self, id: RecordId) -> usize {
        let mut depth = 0;
        let mut current = id;
        while let Some(parent_id) = self.parent_id(current) {
            depth += 1;
            current = parent_id;
        }
        depth
    }

    /// Walk the tree depth-first starting from roots.
    pub fn walk_depth_first(&self) -> TreeWalker<'_, R> {
        TreeWalker::new(self)
    }

    /// Walk the tree depth-first starting from a specific node.
    pub fn walk_from(&self, id: RecordId) -> TreeWalker<'_, R> {
        TreeWalker::from_node(self, id)
    }

    /// Find records matching a predicate.
    pub fn find<F>(&self, predicate: F) -> impl Iterator<Item = (RecordId, &R)>
    where
        F: Fn(&R) -> bool,
    {
        self.records
            .iter()
            .enumerate()
            .filter(move |(_, r)| predicate(r))
            .map(|(i, r)| (RecordId::new(i as u32), r))
    }

    /// Find the first record matching a predicate.
    pub fn find_first<F>(&self, predicate: F) -> Option<(RecordId, &R)>
    where
        F: Fn(&R) -> bool,
    {
        self.records
            .iter()
            .enumerate()
            .find(|(_, r)| predicate(r))
            .map(|(i, r)| (RecordId::new(i as u32), r))
    }

    /// Get the path from a record to the root (inclusive).
    pub fn path_to_root(&self, id: RecordId) -> Vec<RecordId> {
        let mut path = vec![id];
        let mut current = id;
        while let Some(parent_id) = self.parent_id(current) {
            path.push(parent_id);
            current = parent_id;
        }
        path
    }

    /// Consume the tree and return the underlying records.
    pub fn into_records(self) -> Vec<R> {
        self.records
    }

    /// Get a reference to the underlying records slice.
    pub fn as_slice(&self) -> &[R] {
        &self.records
    }

    /// Add a record to the tree.
    ///
    /// Returns the ID of the newly added record.
    pub fn add(&mut self, record: R) -> RecordId {
        let id = RecordId::new(self.records.len() as u32);
        let owner_index = record.owner_index();

        self.records.push(record);

        // Update relationships
        if owner_index < 0 || (owner_index as usize) >= self.records.len() - 1 {
            self.roots.push(id);
        } else {
            let parent_id = RecordId::new(owner_index as u32);
            self.parents.insert(id, parent_id);
            self.children.entry(parent_id).or_default().push(id);
        }

        id
    }

    /// Remove a record from the tree by ID.
    ///
    /// Note: This invalidates all RecordIds after the removed record.
    /// Consider using a different approach for frequent removals.
    pub fn remove(&mut self, id: RecordId) -> Option<R> {
        let index = id.index() as usize;
        if index >= self.records.len() {
            return None;
        }

        let record = self.records.remove(index);

        // Rebuild all relationships (IDs have changed)
        self.rebuild_relationships();

        Some(record)
    }

    /// Move a record to a new parent.
    pub fn reparent(&mut self, id: RecordId, new_parent: Option<RecordId>) {
        // Remove from old parent's children
        if let Some(old_parent_id) = self.parents.remove(&id) {
            if let Some(siblings) = self.children.get_mut(&old_parent_id) {
                siblings.retain(|&child_id| child_id != id);
            }
        }

        // Remove from roots if it was a root
        self.roots.retain(|&root_id| root_id != id);

        // Add to new parent
        if let Some(parent_id) = new_parent {
            self.parents.insert(id, parent_id);
            self.children.entry(parent_id).or_default().push(id);

            // Update the record's owner_index
            if let Some(record) = self.records.get_mut(id.index() as usize) {
                record.set_owner_index(parent_id.index() as i32);
            }
        } else {
            // Make it a root
            self.roots.push(id);
            if let Some(record) = self.records.get_mut(id.index() as usize) {
                record.set_owner_index(-1);
            }
        }
    }
}

impl<R> Default for RecordTree<R> {
    fn default() -> Self {
        RecordTree {
            records: Vec::new(),
            parents: HashMap::new(),
            children: HashMap::new(),
            roots: Vec::new(),
        }
    }
}

/// Iterator over ancestors of a record.
struct AncestorIterator<'a, R: TreeRecord> {
    tree: &'a RecordTree<R>,
    current: Option<RecordId>,
}

impl<'a, R: TreeRecord> Iterator for AncestorIterator<'a, R> {
    type Item = (RecordId, &'a R);

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.current?;
        self.current = self.tree.parent_id(id);
        self.tree.get(id).map(|r| (id, r))
    }
}

/// Iterator over descendants of a record.
struct DescendantIterator<'a, R: TreeRecord> {
    tree: &'a RecordTree<R>,
    stack: Vec<RecordId>,
}

impl<'a, R: TreeRecord> Iterator for DescendantIterator<'a, R> {
    type Item = (RecordId, &'a R);

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.stack.pop()?;

        // Add children to stack for further iteration
        if let Some(children) = self.tree.children.get(&id) {
            self.stack.extend(children.iter().rev());
        }

        self.tree.get(id).map(|r| (id, r))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Default)]
    struct TestRecord {
        owner_index: i32,
        value: String,
    }

    impl TreeRecord for TestRecord {
        fn owner_index(&self) -> i32 {
            self.owner_index
        }

        fn set_owner_index(&mut self, index: i32) {
            self.owner_index = index;
        }
    }

    #[test]
    fn test_build_tree() {
        let records = vec![
            TestRecord {
                owner_index: -1,
                value: "root".to_string(),
            },
            TestRecord {
                owner_index: 0,
                value: "child1".to_string(),
            },
            TestRecord {
                owner_index: 0,
                value: "child2".to_string(),
            },
            TestRecord {
                owner_index: 1,
                value: "grandchild".to_string(),
            },
        ];

        let tree = RecordTree::from_records(records);

        assert_eq!(tree.len(), 4);
        assert_eq!(tree.root_count(), 1);
    }

    #[test]
    fn test_navigation() {
        let records = vec![
            TestRecord {
                owner_index: -1,
                value: "root".to_string(),
            },
            TestRecord {
                owner_index: 0,
                value: "child1".to_string(),
            },
            TestRecord {
                owner_index: 0,
                value: "child2".to_string(),
            },
            TestRecord {
                owner_index: 1,
                value: "grandchild".to_string(),
            },
        ];

        let tree = RecordTree::from_records(records);

        // Test roots
        let roots: Vec<_> = tree.roots().collect();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].1.value, "root");

        // Test children
        let root_id = RecordId::new(0);
        let children: Vec<_> = tree.children(root_id).collect();
        assert_eq!(children.len(), 2);

        // Test parent
        let child_id = RecordId::new(1);
        let parent = tree.parent(child_id);
        assert!(parent.is_some());
        assert_eq!(parent.unwrap().1.value, "root");

        // Test depth
        assert_eq!(tree.depth(RecordId::new(0)), 0);
        assert_eq!(tree.depth(RecordId::new(1)), 1);
        assert_eq!(tree.depth(RecordId::new(3)), 2);
    }

    #[test]
    fn test_walk_depth_first() {
        let records = vec![
            TestRecord {
                owner_index: -1,
                value: "root".to_string(),
            },
            TestRecord {
                owner_index: 0,
                value: "child1".to_string(),
            },
            TestRecord {
                owner_index: 0,
                value: "child2".to_string(),
            },
            TestRecord {
                owner_index: 1,
                value: "grandchild".to_string(),
            },
        ];

        let tree = RecordTree::from_records(records);

        let walked: Vec<_> = tree.walk_depth_first().collect();
        assert_eq!(walked.len(), 4);

        // First should be root at depth 0
        assert_eq!(walked[0].1.value, "root");
        assert_eq!(walked[0].2, 0);
    }

    #[test]
    fn test_ancestors() {
        let records = vec![
            TestRecord {
                owner_index: -1,
                value: "root".to_string(),
            },
            TestRecord {
                owner_index: 0,
                value: "child".to_string(),
            },
            TestRecord {
                owner_index: 1,
                value: "grandchild".to_string(),
            },
        ];

        let tree = RecordTree::from_records(records);

        let ancestors: Vec<_> = tree.ancestors(RecordId::new(2)).collect();
        assert_eq!(ancestors.len(), 2);
        assert_eq!(ancestors[0].1.value, "child");
        assert_eq!(ancestors[1].1.value, "root");
    }
}
