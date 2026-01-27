//! BinaryRecord for binary-format PCB records.

use indexmap::IndexMap;

use super::Value;
use crate::records::pcb::PcbObjectId;

/// A record with dynamic field access for binary format.
///
/// Used for PCB records which are stored in binary format rather than
/// pipe-delimited parameters. Preserves raw bytes for lossless round-tripping.
#[derive(Debug, Clone)]
pub struct BinaryRecord {
    /// Object type ID
    object_id: PcbObjectId,
    /// Original raw binary data
    raw_data: Vec<u8>,
    /// Parsed fields (if known object type)
    fields: Option<IndexMap<String, Value>>,
    /// Whether any fields were modified
    modified: bool,
}

impl BinaryRecord {
    /// Creates a BinaryRecord from raw data and object ID.
    pub fn from_binary(object_id: PcbObjectId, data: Vec<u8>) -> Self {
        BinaryRecord {
            object_id,
            raw_data: data,
            fields: None,
            modified: false,
        }
    }

    /// Creates a BinaryRecord with pre-parsed fields.
    pub fn from_binary_with_fields(
        object_id: PcbObjectId,
        data: Vec<u8>,
        fields: IndexMap<String, Value>,
    ) -> Self {
        BinaryRecord {
            object_id,
            raw_data: data,
            fields: Some(fields),
            modified: false,
        }
    }

    /// Returns the object type ID.
    pub fn object_id(&self) -> PcbObjectId {
        self.object_id
    }

    /// Returns a human-readable type name.
    pub fn type_name(&self) -> &'static str {
        match self.object_id {
            PcbObjectId::Arc => "Arc",
            PcbObjectId::Pad => "Pad",
            PcbObjectId::Via => "Via",
            PcbObjectId::Track => "Track",
            PcbObjectId::Text => "Text",
            PcbObjectId::Fill => "Fill",
            PcbObjectId::Region => "Region",
            PcbObjectId::ComponentBody => "ComponentBody",
            _ => "Unknown",
        }
    }

    /// Returns the raw binary data.
    pub fn raw_data(&self) -> &[u8] {
        &self.raw_data
    }

    /// Returns true if fields have been parsed.
    pub fn has_fields(&self) -> bool {
        self.fields.is_some()
    }

    /// Gets a parsed field value.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.fields.as_ref()?.get(&key.to_uppercase())
    }

    /// Gets a field as an integer.
    pub fn get_int(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(|v| v.as_int())
    }

    /// Gets a field as a float.
    pub fn get_float(&self, key: &str) -> Option<f64> {
        self.get(key).and_then(|v| v.as_float())
    }

    /// Gets a field as a coordinate.
    pub fn get_coord(&self, key: &str) -> Option<crate::types::Coord> {
        self.get(key).and_then(|v| v.as_coord())
    }

    /// Iterates over parsed fields.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.fields
            .iter()
            .flat_map(|f| f.iter())
            .map(|(k, v)| (k.as_str(), v))
    }

    /// Returns true if the record was modified.
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Sets a field value.
    pub fn set(&mut self, key: &str, value: impl Into<Value>) {
        let fields = self.fields.get_or_insert_with(IndexMap::new);
        fields.insert(key.to_uppercase(), value.into());
        self.modified = true;
    }

    /// Converts back to binary data.
    ///
    /// If the record was not modified, returns the original raw data.
    /// If modified, returns the original data (typed serialization requires
    /// using the typed API).
    pub fn to_binary(&self) -> Vec<u8> {
        // For now, always return original data
        // Full binary reconstruction would require type-specific serialization
        self.raw_data.clone()
    }

    /// Returns the size of the binary data.
    pub fn size(&self) -> usize {
        self.raw_data.len()
    }
}

impl Default for BinaryRecord {
    fn default() -> Self {
        BinaryRecord {
            object_id: PcbObjectId::None,
            raw_data: Vec::new(),
            fields: None,
            modified: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_binary() {
        let data = vec![0x01, 0x02, 0x03, 0x04];
        let record = BinaryRecord::from_binary(PcbObjectId::Track, data.clone());

        assert_eq!(record.object_id(), PcbObjectId::Track);
        assert_eq!(record.raw_data(), &data);
        assert!(!record.has_fields());
    }

    #[test]
    fn test_with_fields() {
        let mut fields = IndexMap::new();
        fields.insert("WIDTH".to_string(), Value::Int(1000));

        let record = BinaryRecord::from_binary_with_fields(PcbObjectId::Track, vec![], fields);

        assert!(record.has_fields());
        assert_eq!(record.get_int("WIDTH"), Some(1000));
    }

    #[test]
    fn test_modification() {
        let mut record = BinaryRecord::from_binary(PcbObjectId::Track, vec![]);
        assert!(!record.is_modified());

        record.set("WIDTH", 2000i64);
        assert!(record.is_modified());
    }
}
