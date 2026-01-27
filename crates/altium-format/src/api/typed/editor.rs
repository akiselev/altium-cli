//! EditTransaction for batch editing with commit/rollback.

use std::collections::HashMap;

use crate::api::generic::{GenericRecord, ParamsContainer};

/// Transaction-style editor for batch record modifications.
///
/// Provides commit/rollback semantics for editing records. Changes are
/// staged in memory and only applied when `commit()` is called. If the
/// transaction is dropped without committing, changes are discarded.
pub struct EditTransaction<'a> {
    /// Reference to the container being edited
    container: &'a mut ParamsContainer,
    /// Pending changes (index -> modified record)
    pending: HashMap<usize, GenericRecord>,
    /// Whether the transaction was committed
    committed: bool,
}

impl<'a> EditTransaction<'a> {
    /// Creates a new edit transaction for a container.
    pub fn new(container: &'a mut ParamsContainer) -> Self {
        EditTransaction {
            container,
            pending: HashMap::new(),
            committed: false,
        }
    }

    /// Gets a record for viewing.
    pub fn get(&self, index: usize) -> Option<&GenericRecord> {
        // Return pending version if exists, otherwise original
        if let Some(record) = self.pending.get(&index) {
            Some(record)
        } else {
            self.container.get(index)
        }
    }

    /// Gets a record for editing.
    ///
    /// If this is the first edit to this record, it's cloned into the
    /// pending changes.
    pub fn edit(&mut self, index: usize) -> Option<&mut GenericRecord> {
        if !self.pending.contains_key(&index) {
            if let Some(record) = self.container.get(index) {
                self.pending.insert(index, record.clone());
            } else {
                return None;
            }
        }
        self.pending.get_mut(&index)
    }

    /// Returns the number of pending changes.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Returns indices of records with pending changes.
    pub fn modified_indices(&self) -> Vec<usize> {
        self.pending.keys().copied().collect()
    }

    /// Applies all pending changes to the container.
    pub fn commit(mut self) {
        for (index, record) in self.pending.drain() {
            if let Some(target) = self.container.get_mut(index) {
                *target = record;
            }
        }
        self.committed = true;
    }

    /// Discards all pending changes.
    pub fn rollback(mut self) {
        self.pending.clear();
        self.committed = true; // Mark as handled to prevent drop warning
    }

    /// Returns true if the transaction has pending changes.
    pub fn has_changes(&self) -> bool {
        !self.pending.is_empty()
    }
}

impl<'a> Drop for EditTransaction<'a> {
    fn drop(&mut self) {
        // If not committed, changes are automatically discarded
        // (This is the intended behavior - RAII rollback)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ParameterCollection;

    #[test]
    fn test_edit_and_commit() {
        let params = vec![ParameterCollection::from_string("|RECORD=1|NAME=Original|")];
        let mut container = ParamsContainer::from_params_list("/test", params);

        {
            let mut tx = EditTransaction::new(&mut container);
            let record = tx.edit(0).unwrap();
            record.set("NAME", "Modified");
            tx.commit();
        }

        assert_eq!(container.get(0).unwrap().get_str("NAME"), Some("Modified"));
    }

    #[test]
    fn test_edit_and_rollback() {
        let params = vec![ParameterCollection::from_string("|RECORD=1|NAME=Original|")];
        let mut container = ParamsContainer::from_params_list("/test", params);

        {
            let mut tx = EditTransaction::new(&mut container);
            let record = tx.edit(0).unwrap();
            record.set("NAME", "Modified");
            tx.rollback();
        }

        assert_eq!(container.get(0).unwrap().get_str("NAME"), Some("Original"));
    }

    #[test]
    fn test_implicit_rollback_on_drop() {
        let params = vec![ParameterCollection::from_string("|RECORD=1|NAME=Original|")];
        let mut container = ParamsContainer::from_params_list("/test", params);

        {
            let mut tx = EditTransaction::new(&mut container);
            let record = tx.edit(0).unwrap();
            record.set("NAME", "Modified");
            // Drop without commit
        }

        // Original value should be preserved
        assert_eq!(container.get(0).unwrap().get_str("NAME"), Some("Original"));
    }
}
