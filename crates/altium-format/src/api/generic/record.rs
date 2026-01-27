//! GenericRecord for parameter-format records.

use indexmap::IndexMap;

use super::Value;
use crate::types::ParameterCollection;

/// Entry for a field in a GenericRecord.
#[derive(Debug, Clone)]
struct FieldEntry {
    /// Original key as it appeared in the source
    original_key: String,
    /// Current value
    value: Value,
    /// Whether this field was modified
    modified: bool,
    /// Original raw value (for non-destructive editing)
    original_value: Option<String>,
}

/// A record with dynamic field access for parameter format.
///
/// Preserves original field order and tracks modifications for
/// non-destructive round-trip editing.
#[derive(Debug, Clone)]
pub struct GenericRecord {
    /// RECORD parameter value (schematic record ID)
    record_id: Option<i32>,
    /// Original raw parameter string
    raw_data: String,
    /// Fields in original order
    fields: IndexMap<String, FieldEntry>,
}

impl GenericRecord {
    /// Creates a GenericRecord from a ParameterCollection.
    pub fn from_params(params: &ParameterCollection) -> Self {
        let mut fields = IndexMap::new();
        let mut record_id = None;

        for (key, pv) in params.iter() {
            let value = Value::from_param_value(&pv);

            // Extract record ID
            if key.eq_ignore_ascii_case("RECORD") {
                record_id = value.as_int().map(|i| i as i32);
            }

            fields.insert(
                key.to_uppercase(),
                FieldEntry {
                    original_key: key.to_string(),
                    value,
                    modified: false,
                    original_value: Some(pv.as_str().to_string()),
                },
            );
        }

        GenericRecord {
            record_id,
            raw_data: params.to_string(),
            fields,
        }
    }

    /// Creates an empty GenericRecord with a given record ID.
    pub fn new(record_id: i32) -> Self {
        let mut record = GenericRecord {
            record_id: Some(record_id),
            raw_data: String::new(),
            fields: IndexMap::new(),
        };
        record.set("RECORD", Value::Int(record_id as i64));
        record
    }

    // --- Record Type ---

    /// Returns the record ID (RECORD parameter).
    pub fn record_id(&self) -> Option<i32> {
        self.record_id
    }

    /// Returns a human-readable type name based on record ID.
    pub fn type_name(&self) -> &'static str {
        match self.record_id {
            Some(1) => "Component",
            Some(2) => "Pin",
            Some(3) => "Symbol",
            Some(4) => "Text",
            Some(5) => "Bezier",
            Some(6) => "Polyline",
            Some(7) => "Polygon",
            Some(8) => "Ellipse",
            Some(9) => "Piechart",
            Some(10) => "RectangleBorder",
            Some(11) => "SymbolBorder",
            Some(12) => "GraphicBody",
            Some(13) => "Arc",
            Some(14) => "Line",
            Some(15) => "Rectangle",
            Some(17) => "PowerPort",
            Some(18) => "Port",
            Some(25) => "NetLabel",
            Some(26) => "Bus",
            Some(27) => "Wire",
            Some(28) => "Junction",
            Some(29) => "Image",
            Some(30) => "Sheet",
            Some(31) => "SheetName",
            Some(32) => "FileName",
            Some(33) => "Designator",
            Some(34) => "BusEntry",
            Some(37) => "Template",
            Some(39) => "Parameter",
            Some(41) => "ParameterSet",
            Some(44) => "OffSheet",
            Some(45) => "Harness",
            Some(46) => "HarnessEntry",
            Some(47) => "HarnessType",
            Some(48) => "Implementation",
            Some(215) => "Note",
            Some(226) => "CompileMessage",
            Some(id) => {
                // Can't return dynamic string, return generic name
                if id > 0 { "Record" } else { "Unknown" }
            }
            None => "Unknown",
        }
    }

    // --- Field Access ---

    /// Returns true if the field exists.
    pub fn contains(&self, key: &str) -> bool {
        self.fields.contains_key(&key.to_uppercase())
    }

    /// Gets a field value.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.fields.get(&key.to_uppercase()).map(|e| &e.value)
    }

    /// Gets a field value with a default.
    pub fn get_or<'a>(&'a self, key: &str, default: &'a Value) -> &'a Value {
        self.get(key).unwrap_or(default)
    }

    // --- Typed getters ---

    /// Gets a field as a boolean.
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(|v| v.as_bool())
    }

    /// Gets a field as an integer.
    pub fn get_int(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(|v| v.as_int())
    }

    /// Gets a field as a float.
    pub fn get_float(&self, key: &str) -> Option<f64> {
        self.get(key).and_then(|v| v.as_float())
    }

    /// Gets a field as a string.
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(|v| v.as_str())
    }

    /// Gets a field as a coordinate.
    pub fn get_coord(&self, key: &str) -> Option<crate::types::Coord> {
        self.get(key).and_then(|v| v.as_coord())
    }

    /// Gets a field as a color.
    pub fn get_color(&self, key: &str) -> Option<crate::types::Color> {
        self.get(key).and_then(|v| v.as_color())
    }

    /// Gets a field as a layer.
    pub fn get_layer(&self, key: &str) -> Option<crate::types::Layer> {
        self.get(key).and_then(|v| v.as_layer())
    }

    // --- Iteration ---

    /// Iterates over all fields in original order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.fields.iter().map(|(k, e)| (k.as_str(), &e.value))
    }

    /// Returns field keys in original order.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.fields.keys().map(|k| k.as_str())
    }

    /// Returns field values in original order.
    pub fn values(&self) -> impl Iterator<Item = &Value> {
        self.fields.values().map(|e| &e.value)
    }

    /// Returns the number of fields.
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// Returns true if there are no fields.
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    // --- Modification ---

    /// Sets a field value, preserving position if it exists.
    pub fn set(&mut self, key: &str, value: impl Into<Value>) {
        let upper_key = key.to_uppercase();
        let value = value.into();

        // Update record_id if setting RECORD
        if upper_key == "RECORD" {
            self.record_id = value.as_int().map(|i| i as i32);
        }

        if let Some(entry) = self.fields.get_mut(&upper_key) {
            entry.value = value;
            entry.modified = true;
        } else {
            self.fields.insert(
                upper_key,
                FieldEntry {
                    original_key: key.to_string(),
                    value,
                    modified: true,
                    original_value: None,
                },
            );
        }
    }

    /// Removes a field and returns its value.
    pub fn remove(&mut self, key: &str) -> Option<Value> {
        self.fields
            .swap_remove(&key.to_uppercase())
            .map(|e| e.value)
    }

    /// Returns true if any fields were modified.
    pub fn is_modified(&self) -> bool {
        self.fields.values().any(|e| e.modified)
    }

    /// Returns the names of modified fields.
    pub fn modified_fields(&self) -> Vec<&str> {
        self.fields
            .iter()
            .filter(|(_, e)| e.modified)
            .map(|(k, _)| k.as_str())
            .collect()
    }

    /// Resets all modifications, restoring original values.
    pub fn reset(&mut self) {
        for entry in self.fields.values_mut() {
            if entry.modified {
                if let Some(ref original) = entry.original_value {
                    // Re-parse original value
                    entry.value = Value::String(original.clone());
                    entry.modified = false;
                }
            }
        }
    }

    // --- Serialization ---

    /// Converts back to a ParameterCollection.
    pub fn to_params(&self) -> ParameterCollection {
        let mut params = ParameterCollection::new();

        for (_, entry) in &self.fields {
            let value_str = match &entry.value {
                Value::Null => continue,
                Value::Bool(b) => if *b { "T" } else { "F" }.to_string(),
                Value::Int(i) => i.to_string(),
                Value::Float(f) => format!("{}", f),
                Value::String(s) => s.clone(),
                Value::Coord(c) => format!("{}mil", c.to_mils()),
                Value::Color(c) => c.to_win32().to_string(),
                Value::Layer(l) => l.0.to_string(),
                Value::List(l) => l
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
                Value::Binary(_) => continue, // Skip binary in params
            };

            params.add(&entry.original_key, &value_str);
        }

        params
    }

    /// Converts to a parameter string.
    pub fn to_params_string(&self) -> String {
        self.to_params().to_string()
    }

    /// Returns the raw original data.
    pub fn raw_data(&self) -> &str {
        &self.raw_data
    }

    // --- Hierarchy ---

    /// Gets the owner index (OWNERINDEX parameter).
    pub fn owner_index(&self) -> i32 {
        self.get_int("OWNERINDEX").unwrap_or(-1) as i32
    }

    /// Sets the owner index.
    pub fn set_owner_index(&mut self, index: i32) {
        self.set("OWNERINDEX", Value::Int(index as i64));
    }
}

impl Default for GenericRecord {
    fn default() -> Self {
        GenericRecord {
            record_id: None,
            raw_data: String::new(),
            fields: IndexMap::new(),
        }
    }
}

impl std::ops::Index<&str> for GenericRecord {
    type Output = Value;

    fn index(&self, key: &str) -> &Value {
        static NULL: Value = Value::Null;
        self.get(key).unwrap_or(&NULL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_params() {
        let params = ParameterCollection::from_string("|RECORD=1|NAME=Test|VALUE=42|");
        let record = GenericRecord::from_params(&params);

        assert_eq!(record.record_id(), Some(1));
        assert_eq!(record.get_str("NAME"), Some("Test"));
        assert_eq!(record.get_int("VALUE"), Some(42));
    }

    #[test]
    fn test_modification_tracking() {
        let params = ParameterCollection::from_string("|RECORD=1|NAME=Test|");
        let mut record = GenericRecord::from_params(&params);

        assert!(!record.is_modified());

        record.set("NAME", "Modified");
        assert!(record.is_modified());
        assert_eq!(record.modified_fields(), vec!["NAME"]);
    }

    #[test]
    fn test_order_preservation() {
        let params = ParameterCollection::from_string("|A=1|B=2|C=3|");
        let record = GenericRecord::from_params(&params);

        let keys: Vec<_> = record.keys().collect();
        assert_eq!(keys, vec!["A", "B", "C"]);
    }

    #[test]
    fn test_index_operator() {
        let params = ParameterCollection::from_string("|NAME=Test|");
        let record = GenericRecord::from_params(&params);

        assert_eq!(record["NAME"].as_str(), Some("Test"));
        assert!(record["MISSING"].is_null());
    }
}
