//! Depth-first tree walker iterator.

use super::{RecordId, RecordTree, TreeRecord};

/// Depth-first iterator over tree nodes.
///
/// Yields (RecordId, &Record, depth) tuples in pre-order traversal.
pub struct TreeWalker<'a, R> {
    tree: &'a RecordTree<R>,
    /// Stack of (RecordId, depth) pairs to visit.
    stack: Vec<(RecordId, usize)>,
}

impl<'a, R: TreeRecord> TreeWalker<'a, R> {
    /// Create a new walker starting from all roots.
    pub fn new(tree: &'a RecordTree<R>) -> Self {
        // Push roots in reverse order so first root is processed first
        let stack = tree.roots.iter().rev().map(|&id| (id, 0)).collect();

        TreeWalker { tree, stack }
    }

    /// Create a walker starting from a specific node.
    pub fn from_node(tree: &'a RecordTree<R>, start: RecordId) -> Self {
        let depth = tree.depth(start);
        TreeWalker {
            tree,
            stack: vec![(start, depth)],
        }
    }
}

impl<'a, R: TreeRecord> Iterator for TreeWalker<'a, R> {
    type Item = (RecordId, &'a R, usize);

    fn next(&mut self) -> Option<Self::Item> {
        let (id, depth) = self.stack.pop()?;

        // Get the record
        let record = self.tree.get(id)?;

        // Push children in reverse order for correct traversal order
        if let Some(children) = self.tree.children.get(&id) {
            for &child_id in children.iter().rev() {
                self.stack.push((child_id, depth + 1));
            }
        }

        Some((id, record, depth))
    }
}

/// Breadth-first iterator over tree nodes.
///
/// Yields (RecordId, &Record, depth) tuples level by level.
pub struct BreadthFirstWalker<'a, R> {
    tree: &'a RecordTree<R>,
    /// Queue of (RecordId, depth) pairs to visit.
    queue: std::collections::VecDeque<(RecordId, usize)>,
}

impl<'a, R: TreeRecord> BreadthFirstWalker<'a, R> {
    /// Create a new breadth-first walker starting from all roots.
    pub fn new(tree: &'a RecordTree<R>) -> Self {
        let queue = tree.roots.iter().map(|&id| (id, 0)).collect();

        BreadthFirstWalker { tree, queue }
    }

    /// Create a walker starting from a specific node.
    pub fn from_node(tree: &'a RecordTree<R>, start: RecordId) -> Self {
        let depth = tree.depth(start);
        let mut queue = std::collections::VecDeque::new();
        queue.push_back((start, depth));
        BreadthFirstWalker { tree, queue }
    }
}

impl<'a, R: TreeRecord> Iterator for BreadthFirstWalker<'a, R> {
    type Item = (RecordId, &'a R, usize);

    fn next(&mut self) -> Option<Self::Item> {
        let (id, depth) = self.queue.pop_front()?;

        // Get the record
        let record = self.tree.get(id)?;

        // Add children to the back of the queue
        if let Some(children) = self.tree.children.get(&id) {
            for &child_id in children.iter() {
                self.queue.push_back((child_id, depth + 1));
            }
        }

        Some((id, record, depth))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Default)]
    struct TestRecord {
        owner_index: i32,
        name: String,
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
    fn test_depth_first_walk() {
        // Build a tree:
        //   0 (root)
        //   ├── 1
        //   │   └── 3
        //   └── 2
        let records = vec![
            TestRecord {
                owner_index: -1,
                name: "root".to_string(),
            },
            TestRecord {
                owner_index: 0,
                name: "child1".to_string(),
            },
            TestRecord {
                owner_index: 0,
                name: "child2".to_string(),
            },
            TestRecord {
                owner_index: 1,
                name: "grandchild".to_string(),
            },
        ];

        let tree = RecordTree::from_records(records);
        let walked: Vec<_> = tree
            .walk_depth_first()
            .map(|(_, r, d)| (r.name.clone(), d))
            .collect();

        // Depth-first: root, child1, grandchild, child2
        assert_eq!(
            walked,
            vec![
                ("root".to_string(), 0),
                ("child1".to_string(), 1),
                ("grandchild".to_string(), 2),
                ("child2".to_string(), 1),
            ]
        );
    }

    #[test]
    fn test_breadth_first_walk() {
        let records = vec![
            TestRecord {
                owner_index: -1,
                name: "root".to_string(),
            },
            TestRecord {
                owner_index: 0,
                name: "child1".to_string(),
            },
            TestRecord {
                owner_index: 0,
                name: "child2".to_string(),
            },
            TestRecord {
                owner_index: 1,
                name: "grandchild".to_string(),
            },
        ];

        let tree = RecordTree::from_records(records);
        let walker = BreadthFirstWalker::new(&tree);
        let walked: Vec<_> = walker.map(|(_, r, d)| (r.name.clone(), d)).collect();

        // Breadth-first: root, child1, child2, grandchild
        assert_eq!(
            walked,
            vec![
                ("root".to_string(), 0),
                ("child1".to_string(), 1),
                ("child2".to_string(), 1),
                ("grandchild".to_string(), 2),
            ]
        );
    }
}
