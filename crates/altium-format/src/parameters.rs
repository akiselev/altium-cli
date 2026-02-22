//! Parameter collection for Altium's pipe-delimited key-value storage.
//!
//! Altium stores most schematic and library data as pipe-delimited parameters.
//! Example: `|RECORD=1|LIBREFERENCE=Component|COMPONENTDESCRIPTION=Resistor|`
//!
//! Key implementation details:
//! - Uses IndexMap to preserve insertion order
//! - Nesting level changes separator (`|` at level 0, `` ` `` at level 1)
//! - Handles `%UTF8%` prefix for Unicode values

use crate::error::AltiumError;
use encoding_rs::WINDOWS_1252;
use indexmap::IndexMap;
use std::fmt;

/// Entry separators for different nesting levels.
const ENTRY_SEPARATORS: &[char] = &['|', '`'];

/// Key-value separator.
const KEY_VALUE_SEPARATOR: char = '=';

/// UTF-8 prefix marker.
const UTF8_PREFIX: &str = "%UTF8%";

/// Boolean true values.
const TRUE_VALUES: &[&str] = &["T", "TRUE"];

/// Boolean false values.
const FALSE_VALUES: &[&str] = &["F", "FALSE"];

/// Value of a parameter with typed conversion methods.
#[derive(Clone, Debug, Default)]
pub struct ParameterValue {
    data: String,
    level: usize,
}

impl ParameterValue {
    /// Creates a new parameter value.
    pub fn new(data: String, level: usize) -> Self {
        ParameterValue { data, level }
    }

    /// Gets the raw string value.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.data
    }

    /// Gets the string value, or a default if empty.
    pub fn as_string_or(&self, default: &str) -> String {
        if self.data.is_empty() {
            default.to_string()
        } else {
            self.data.clone()
        }
    }

    /// Parses the value as an integer.
    pub fn as_int(&self) -> Result<i32, std::num::ParseIntError> {
        self.data.trim().trim_matches('\0').parse()
    }

    /// Gets the value as an integer, or a default on parse failure.
    pub fn as_int_or(&self, default: i32) -> i32 {
        self.data
            .trim()
            .trim_matches('\0')
            .parse()
            .unwrap_or(default)
    }

    /// Parses the value as a double.
    pub fn as_double(&self) -> Result<f64, std::num::ParseFloatError> {
        self.data.trim().trim_matches('\0').parse()
    }

    /// Gets the value as a double, or a default on parse failure.
    pub fn as_double_or(&self, default: f64) -> f64 {
        self.data
            .trim()
            .trim_matches('\0')
            .parse()
            .unwrap_or(default)
    }

    /// Parses the value as a boolean.
    ///
    /// Accepts: "T", "TRUE" (true), "F", "FALSE" (false)
    pub fn as_bool(&self) -> Result<bool, &'static str> {
        let s = self.data.trim().trim_matches('\0').to_uppercase();
        if TRUE_VALUES.contains(&s.as_str()) {
            Ok(true)
        } else if FALSE_VALUES.contains(&s.as_str()) || s.is_empty() {
            Ok(false)
        } else {
            Err("Invalid boolean value")
        }
    }

    /// Gets the value as a boolean, or a default on parse failure.
    pub fn as_bool_or(&self, default: bool) -> bool {
        self.as_bool().unwrap_or(default)
    }

    /// Parses the value as a nested ParameterCollection.
    pub fn as_parameters(&self) -> ParameterCollection {
        ParameterCollection::from_string_with_level(&self.data, self.level + 1)
    }

    /// Splits the value by the list separator and returns string items.
    pub fn as_string_list(&self) -> Vec<String> {
        self.as_list_impl().map(|s| s.to_string()).collect()
    }

    /// Splits the value by the list separator and returns integer items.
    pub fn as_int_list(&self) -> Result<Vec<i32>, AltiumError> {
        self.as_list_impl()
            .map(|s| {
                s.trim()
                    .parse()
                    .map_err(|e| AltiumError::Parse(format!("invalid integer '{}': {}", s.trim(), e)))
            })
            .collect()
    }

    /// Splits the value by the list separator and returns double items.
    pub fn as_double_list(&self) -> Result<Vec<f64>, AltiumError> {
        self.as_list_impl()
            .map(|s| {
                s.trim()
                    .parse()
                    .map_err(|e| AltiumError::Parse(format!("invalid float '{}': {}", s.trim(), e)))
            })
            .collect()
    }

    /// Helper to split by comma.
    fn as_list_impl(&self) -> impl Iterator<Item = &str> {
        self.data.split(',').filter(|s| !s.is_empty())
    }

    /// Returns true if this value contains nested parameters.
    pub fn is_parameters(&self) -> bool {
        let sep = ENTRY_SEPARATORS.get(self.level + 1).copied().unwrap_or('`');
        self.data.contains(sep)
    }

    /// Returns true if this value is a list (contains commas).
    pub fn is_list(&self) -> bool {
        self.data.contains(',')
    }
}

impl fmt::Display for ParameterValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.data)
    }
}

/// Collection of key-value parameters.
///
/// Preserves insertion order and supports nested parameter strings.
#[derive(Clone, Debug, Default)]
pub struct ParameterCollection {
    /// Raw data string (optional, for debugging).
    #[allow(dead_code)] // Preserved for debugging and round-trip support
    data: Option<String>,
    /// Nesting level (affects separator character).
    level: usize,
    /// Keys in insertion order.
    keys: Vec<String>,
    /// Key-value storage (keys are uppercase).
    parameters: IndexMap<String, String>,
    /// Whether to use long boolean format ("TRUE"/"FALSE" vs "T"/"F").
    use_long_booleans: bool,
    /// Raw suffix appended after normal params (for duplicate keys like ISHIDDEN).
    raw_suffix: Option<String>,
    /// Parsed entries in encounter order (preserves duplicate keys).
    parsed_entries: Vec<(String, String)>,
    /// Whether to serialize using `parsed_entries` for lossless round-trip.
    preserve_entry_order: bool,
    /// Whether the parsed input ended with the entry separator.
    had_trailing_separator: bool,
}

impl ParameterCollection {
    /// Creates an empty parameter collection.
    pub fn new() -> Self {
        ParameterCollection::default()
    }

    /// Creates a parameter collection from a string.
    pub fn from_string(data: &str) -> Self {
        Self::from_string_with_level(data, 0)
    }

    /// Creates a parameter collection from a string at a specific nesting level.
    pub fn from_string_with_level(data: &str, level: usize) -> Self {
        let mut collection = ParameterCollection {
            data: Some(data.to_string()),
            level,
            keys: Vec::new(),
            parameters: IndexMap::new(),
            use_long_booleans: false,
            raw_suffix: None,
            parsed_entries: Vec::new(),
            preserve_entry_order: true,
            had_trailing_separator: data
                .ends_with(ENTRY_SEPARATORS.get(level).copied().unwrap_or('|')),
        };
        collection.parse_data(data);
        collection
    }

    /// Parses the data string into key-value pairs.
    fn parse_data(&mut self, data: &str) {
        let separator = ENTRY_SEPARATORS.get(self.level).copied().unwrap_or('|');
        let mut ignored: std::collections::HashSet<String> = std::collections::HashSet::new();

        for entry in data.split(separator).filter(|s| !s.is_empty()) {
            let entry = entry.trim_end_matches(&['\r', '\n'] as &[char]);

            let (key, value) = if let Some(pos) = entry.find(KEY_VALUE_SEPARATOR) {
                let (k, v) = entry.split_at(pos);
                (k.to_string(), v[1..].to_string())
            } else {
                (String::new(), entry.to_string())
            };

            // Always preserve the original parsed entry verbatim for lossless
            // round-trip when preserve_entry_order is enabled.
            if self.preserve_entry_order {
                self.parsed_entries.push((key.clone(), value.clone()));
            }

            // Skip if already processed as UTF8
            let upper_key = key.to_uppercase();
            if ignored.contains(&upper_key) {
                continue;
            }

            // Handle UTF8 prefix
            let (final_key, final_value) = if let Some(stripped) = key.strip_prefix(UTF8_PREFIX) {
                let real_key = stripped.to_string();
                // Decode UTF-8 from Windows-1252 interpretation
                let decoded = Self::decode_utf8_from_win1252(&value);
                ignored.insert(real_key.to_uppercase());
                (real_key, decoded)
            } else {
                (key, value)
            };

            self.add_internal(&final_key, &final_value);
        }
    }

    /// Decodes a UTF-8 string that was stored as Windows-1252.
    fn decode_utf8_from_win1252(s: &str) -> String {
        let (bytes, _, _) = WINDOWS_1252.encode(s);
        match std::str::from_utf8(bytes.as_ref()) {
            Ok(decoded) => decoded.to_string(),
            Err(_) => s.to_string(),
        }
    }

    /// Internal method to add a key-value pair.
    fn add_internal(&mut self, key: &str, value: &str) {
        let upper_key = key.to_uppercase();
        if !self.parameters.contains_key(&upper_key) {
            self.keys.push(upper_key.clone());
        }
        self.parameters.insert(upper_key, value.to_string());
    }

    /// Returns true if the collection contains the given key.
    pub fn contains(&self, key: &str) -> bool {
        self.parameters.contains_key(&key.to_uppercase())
    }

    /// Gets a parameter value by key.
    pub fn get(&self, key: &str) -> Option<ParameterValue> {
        self.parameters
            .get(&key.to_uppercase())
            .map(|v| ParameterValue::new(v.clone(), self.level))
    }

    /// Gets a parameter value by key, or returns a default value.
    pub fn get_or(&self, key: &str, default: &str) -> ParameterValue {
        self.get(key)
            .unwrap_or_else(|| ParameterValue::new(default.to_string(), self.level))
    }

    /// Gets the value at the given index.
    pub fn get_at(&self, index: usize) -> Option<(&str, ParameterValue)> {
        self.keys.get(index).and_then(|k| {
            self.parameters
                .get(k)
                .map(|v| (k.as_str(), ParameterValue::new(v.clone(), self.level)))
        })
    }

    /// Returns the index of a key, or None if not found.
    pub fn index_of(&self, key: &str) -> Option<usize> {
        let upper = key.to_uppercase();
        self.keys.iter().position(|k| k == &upper)
    }

    /// Adds a string value.
    pub fn add(&mut self, key: &str, value: &str) {
        self.preserve_entry_order = false;
        self.had_trailing_separator = false;
        self.add_internal(key, value);
    }

    /// Adds an integer value.
    pub fn add_int(&mut self, key: &str, value: i32) {
        self.preserve_entry_order = false;
        self.had_trailing_separator = false;
        if value != 0 {
            self.add_internal(key, &value.to_string());
        }
    }

    /// Adds a double value with specified decimal places.
    pub fn add_double(&mut self, key: &str, value: f64, decimals: usize) {
        self.preserve_entry_order = false;
        self.had_trailing_separator = false;
        if value != 0.0 {
            let formatted = format!("{:.prec$}", value, prec = decimals);
            self.add_internal(key, &formatted);
        }
    }

    /// Adds a boolean value.
    pub fn add_bool(&mut self, key: &str, value: bool) {
        self.preserve_entry_order = false;
        self.had_trailing_separator = false;
        if value {
            let s = if self.use_long_booleans { "TRUE" } else { "T" };
            self.add_internal(key, s);
        }
    }

    /// Removes a parameter by key.
    pub fn remove(&mut self, key: &str) {
        self.preserve_entry_order = false;
        self.had_trailing_separator = false;
        let upper = key.to_uppercase();
        self.parameters.swap_remove(&upper);
        self.keys.retain(|k| k != &upper);
    }

    /// Returns the number of parameters.
    pub fn len(&self) -> usize {
        self.parameters.len()
    }

    /// Returns true if empty.
    pub fn is_empty(&self) -> bool {
        self.parameters.is_empty()
    }

    /// Returns an iterator over (key, value) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&str, ParameterValue)> {
        self.keys.iter().filter_map(move |k| {
            self.parameters
                .get(k)
                .map(|v| (k.as_str(), ParameterValue::new(v.clone(), self.level)))
        })
    }

    /// Returns the nesting level.
    pub fn level(&self) -> usize {
        self.level
    }

    /// Sets whether to use long boolean format.
    pub fn set_use_long_booleans(&mut self, value: bool) {
        self.use_long_booleans = value;
    }

    /// Appends a raw pipe-delimited suffix (for duplicate keys that can't be
    /// represented in the key-value map, e.g. Altium's duplicate ISHIDDEN=T).
    pub fn add_raw_suffix(&mut self, suffix: &str) {
        match &mut self.raw_suffix {
            Some(existing) => existing.push_str(suffix),
            None => self.raw_suffix = Some(suffix.to_string()),
        }
    }

    /// Converts the collection back to a parameter string.
    pub fn to_param_string(&self) -> String {
        let separator = ENTRY_SEPARATORS.get(self.level).copied().unwrap_or('|');
        let mut result = String::new();

        if self.preserve_entry_order {
            for (key, value) in &self.parsed_entries {
                result.push(separator);
                result.push_str(key);
                result.push(KEY_VALUE_SEPARATOR);
                result.push_str(value);
            }
            if self.had_trailing_separator {
                result.push(separator);
            }
            if let Some(suffix) = &self.raw_suffix {
                result.push_str(suffix);
            }
            return result;
        }

        for (key, value) in self.iter() {
            result.push(separator);
            result.push_str(key);
            result.push(KEY_VALUE_SEPARATOR);
            result.push_str(&value.data);
        }

        if let Some(suffix) = &self.raw_suffix {
            result.push_str(suffix);
        }

        result
    }
}

impl fmt::Display for ParameterCollection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_param_string())
    }
}

impl<'a> IntoIterator for &'a ParameterCollection {
    type Item = (&'a str, ParameterValue);
    type IntoIter = Box<dyn Iterator<Item = (&'a str, ParameterValue)> + 'a>;

    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.keys.iter().filter_map(move |k| {
            self.parameters
                .get(k)
                .map(|v| (k.as_str(), ParameterValue::new(v.clone(), self.level)))
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_parameters() {
        let data = "|RECORD=1|LIBREFERENCE=Resistor|VALUE=10k|";
        let params = ParameterCollection::from_string(data);

        assert_eq!(params.len(), 3);
        assert_eq!(params.get("RECORD").unwrap().as_int_or(0), 1);
        assert_eq!(params.get("LIBREFERENCE").unwrap().as_str(), "Resistor");
        assert_eq!(params.get("VALUE").unwrap().as_str(), "10k");
    }

    #[test]
    fn test_boolean_values() {
        let data = "|VISIBLE=T|LOCKED=FALSE|";
        let params = ParameterCollection::from_string(data);

        assert!(params.get("VISIBLE").unwrap().as_bool_or(false));
        assert!(!params.get("LOCKED").unwrap().as_bool_or(true));
    }

    #[test]
    fn test_integer_list() {
        let data = "|POINTS=1,2,3,4,5|";
        let params = ParameterCollection::from_string(data);

        let list = params.get("POINTS").unwrap().as_int_list().unwrap();
        assert_eq!(list, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_to_param_string() {
        let mut params = ParameterCollection::new();
        params.add("RECORD", "1");
        params.add("NAME", "Test");

        let s = params.to_param_string();
        assert!(s.contains("|RECORD=1"));
        assert!(s.contains("|NAME=Test"));
    }

    #[test]
    fn test_duplicate_keys_preserved_on_parse_serialize() {
        let raw = "|A=1|A=2|B=3|";
        let params = ParameterCollection::from_string(raw);
        assert_eq!(params.to_param_string(), raw);
    }
}
