//! Container types for collections of records.

use crate::api::cfb::Block;
use crate::records::pcb::PcbObjectId;
use crate::types::ParameterCollection;

use super::{BinaryRecord, GenericRecord};

/// Container for parameter-format records.
///
/// Used for schematic files where records are pipe-delimited parameters.
#[derive(Debug, Clone)]
pub struct ParamsContainer {
    /// Source stream path
    source: String,
    /// Records in order
    records: Vec<GenericRecord>,
}

impl ParamsContainer {
    /// Creates a container from a stream path and blocks.
    pub fn from_blocks(source: &str, blocks: &[Block]) -> Self {
        let mut records = Vec::with_capacity(blocks.len());

        for block in blocks {
            if let Some(params) = block.as_params() {
                records.push(GenericRecord::from_params(&params));
            }
        }

        ParamsContainer {
            source: source.to_string(),
            records,
        }
    }

    /// Creates a container from raw parameter strings.
    pub fn from_params_list(source: &str, params_list: Vec<ParameterCollection>) -> Self {
        let records = params_list.iter().map(GenericRecord::from_params).collect();

        ParamsContainer {
            source: source.to_string(),
            records,
        }
    }

    /// Returns the source stream path.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Returns the number of records.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Returns true if there are no records.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Gets a record by index.
    pub fn get(&self, index: usize) -> Option<&GenericRecord> {
        self.records.get(index)
    }

    /// Gets a mutable record by index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut GenericRecord> {
        self.records.get_mut(index)
    }

    /// Iterates over all records.
    pub fn iter(&self) -> impl Iterator<Item = &GenericRecord> {
        self.records.iter()
    }

    /// Iterates mutably over all records.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut GenericRecord> {
        self.records.iter_mut()
    }

    /// Filters records by record ID.
    pub fn filter_by_type(&self, record_id: i32) -> impl Iterator<Item = &GenericRecord> {
        self.records
            .iter()
            .filter(move |r| r.record_id() == Some(record_id))
    }

    /// Finds the first record matching a predicate.
    pub fn find<F>(&self, pred: F) -> Option<&GenericRecord>
    where
        F: Fn(&GenericRecord) -> bool,
    {
        self.records.iter().find(|r| pred(r))
    }

    /// Finds all records matching a predicate.
    pub fn find_all<F>(&self, pred: F) -> Vec<&GenericRecord>
    where
        F: Fn(&GenericRecord) -> bool,
    {
        self.records.iter().filter(|r| pred(r)).collect()
    }

    /// Returns true if any records were modified.
    pub fn is_modified(&self) -> bool {
        self.records.iter().any(|r| r.is_modified())
    }

    /// Adds a record to the container.
    pub fn push(&mut self, record: GenericRecord) {
        self.records.push(record);
    }

    /// Removes a record by index.
    pub fn remove(&mut self, index: usize) -> Option<GenericRecord> {
        if index < self.records.len() {
            Some(self.records.remove(index))
        } else {
            None
        }
    }

    /// Converts all records back to parameter collections.
    pub fn to_params_list(&self) -> Vec<ParameterCollection> {
        self.records.iter().map(|r| r.to_params()).collect()
    }
}

impl IntoIterator for ParamsContainer {
    type Item = GenericRecord;
    type IntoIter = std::vec::IntoIter<GenericRecord>;

    fn into_iter(self) -> Self::IntoIter {
        self.records.into_iter()
    }
}

impl<'a> IntoIterator for &'a ParamsContainer {
    type Item = &'a GenericRecord;
    type IntoIter = std::slice::Iter<'a, GenericRecord>;

    fn into_iter(self) -> Self::IntoIter {
        self.records.iter()
    }
}

/// Container for binary-format records.
///
/// Used for PCB files where records are size-prefixed binary blocks.
#[derive(Debug, Clone)]
pub struct BinaryContainer {
    /// Source stream path
    source: String,
    /// Records in order
    records: Vec<BinaryRecord>,
}

impl BinaryContainer {
    /// Creates a container from raw binary blocks.
    ///
    /// Each block is expected to have object_id as the first byte.
    pub fn from_blocks(source: &str, blocks: &[Block]) -> Self {
        let mut records = Vec::with_capacity(blocks.len());

        for block in blocks {
            if block.data.is_empty() {
                continue;
            }

            let object_id = PcbObjectId::from_byte(block.data[0]);
            records.push(BinaryRecord::from_binary(object_id, block.data.clone()));
        }

        BinaryContainer {
            source: source.to_string(),
            records,
        }
    }

    /// Creates an empty container.
    pub fn new(source: &str) -> Self {
        BinaryContainer {
            source: source.to_string(),
            records: Vec::new(),
        }
    }

    /// Returns the source stream path.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Returns the number of records.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Returns true if there are no records.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Gets a record by index.
    pub fn get(&self, index: usize) -> Option<&BinaryRecord> {
        self.records.get(index)
    }

    /// Gets a mutable record by index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut BinaryRecord> {
        self.records.get_mut(index)
    }

    /// Iterates over all records.
    pub fn iter(&self) -> impl Iterator<Item = &BinaryRecord> {
        self.records.iter()
    }

    /// Iterates mutably over all records.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut BinaryRecord> {
        self.records.iter_mut()
    }

    /// Filters records by object ID.
    pub fn filter_by_type(&self, object_id: PcbObjectId) -> impl Iterator<Item = &BinaryRecord> {
        self.records
            .iter()
            .filter(move |r| r.object_id() == object_id)
    }

    /// Returns true if any records were modified.
    pub fn is_modified(&self) -> bool {
        self.records.iter().any(|r| r.is_modified())
    }

    /// Adds a record to the container.
    pub fn push(&mut self, record: BinaryRecord) {
        self.records.push(record);
    }

    /// Removes a record by index.
    pub fn remove(&mut self, index: usize) -> Option<BinaryRecord> {
        if index < self.records.len() {
            Some(self.records.remove(index))
        } else {
            None
        }
    }

    /// Converts all records back to binary data.
    pub fn to_binary(&self) -> Vec<Vec<u8>> {
        self.records.iter().map(|r| r.to_binary()).collect()
    }
}

impl IntoIterator for BinaryContainer {
    type Item = BinaryRecord;
    type IntoIter = std::vec::IntoIter<BinaryRecord>;

    fn into_iter(self) -> Self::IntoIter {
        self.records.into_iter()
    }
}

impl<'a> IntoIterator for &'a BinaryContainer {
    type Item = &'a BinaryRecord;
    type IntoIter = std::slice::Iter<'a, BinaryRecord>;

    fn into_iter(self) -> Self::IntoIter {
        self.records.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::cfb::Block;

    #[test]
    fn test_params_container_from_blocks() {
        let blocks = vec![
            Block {
                offset: 0,
                size: 20,
                flags: 0,
                data: b"|RECORD=1|NAME=A|\0".to_vec(),
            },
            Block {
                offset: 24,
                size: 20,
                flags: 0,
                data: b"|RECORD=2|NAME=B|\0".to_vec(),
            },
        ];

        let container = ParamsContainer::from_blocks("/test", &blocks);
        assert_eq!(container.len(), 2);
        assert_eq!(container.get(0).unwrap().record_id(), Some(1));
        assert_eq!(container.get(1).unwrap().record_id(), Some(2));
    }

    #[test]
    fn test_filter_by_type() {
        let params = vec![
            ParameterCollection::from_string("|RECORD=1|"),
            ParameterCollection::from_string("|RECORD=2|"),
            ParameterCollection::from_string("|RECORD=1|"),
        ];
        let container = ParamsContainer::from_params_list("/test", params);

        let record1s: Vec<_> = container.filter_by_type(1).collect();
        assert_eq!(record1s.len(), 2);
    }

    #[test]
    fn test_modification_tracking() {
        let params = vec![ParameterCollection::from_string("|RECORD=1|NAME=Test|")];
        let mut container = ParamsContainer::from_params_list("/test", params);

        assert!(!container.is_modified());

        container.get_mut(0).unwrap().set("NAME", "Modified");
        assert!(container.is_modified());
    }
}
