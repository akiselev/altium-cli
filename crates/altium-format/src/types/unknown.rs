//! Unknown field preservation for non-destructive editing.
//!
//! This module provides types for preserving unknown parameters and binary data
//! during round-trip parsing, enabling non-destructive editing of Altium files.

use super::ParameterCollection;

/// Container for unknown/unrecognized fields during parsing.
///
/// When parsing a record, any parameters that are not explicitly handled by
/// the record type are collected here. During serialization, these unknown
/// fields are written back, preserving the original file content.
#[derive(Debug, Clone, Default)]
pub struct UnknownFields {
    /// Unknown parameters (for parameter-based formats).
    params: ParameterCollection,
    /// Unknown binary data (for binary formats).
    binary: Vec<u8>,
    /// Original parameter order (for exact round-trip fidelity).
    param_order: Vec<String>,
}

impl UnknownFields {
    /// Create an empty UnknownFields container.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create from remaining parameters after known fields have been extracted.
    ///
    /// This compares the original parameter collection against a list of known
    /// keys, collecting any parameters that weren't in the known list.
    pub fn from_remaining_params(original: &ParameterCollection, known_keys: &[&str]) -> Self {
        Self::from_remaining_params_with_prefixes(original, known_keys, &[])
    }

    /// Create from remaining parameters with indexed prefix exclusion.
    ///
    /// In addition to exact key matching, this method also excludes any parameters
    /// that start with the given prefixes followed by a digit (e.g., "X1", "X2" for prefix "X").
    pub fn from_remaining_params_with_prefixes(
        original: &ParameterCollection,
        known_keys: &[&str],
        indexed_prefixes: &[&str],
    ) -> Self {
        let mut params = ParameterCollection::new();
        let mut param_order = Vec::new();

        // Convert known keys to uppercase for case-insensitive comparison
        let known_upper: Vec<String> = known_keys.iter().map(|k| k.to_uppercase()).collect();

        // Convert prefixes to uppercase
        let prefixes_upper: Vec<String> =
            indexed_prefixes.iter().map(|p| p.to_uppercase()).collect();

        for (key, value) in original.iter() {
            let key_upper = key.to_uppercase();

            // Check if it's an exact known key
            if known_upper.contains(&key_upper) {
                continue;
            }

            // Check if it matches an indexed prefix pattern (PREFIX + digit)
            let is_indexed = prefixes_upper.iter().any(|prefix| {
                if key_upper.starts_with(prefix) {
                    let suffix = &key_upper[prefix.len()..];
                    // Check if suffix starts with a digit (X1, X2, etc.)
                    // Also handle _FRAC suffix (X1_FRAC)
                    suffix
                        .chars()
                        .next()
                        .map(|c| c.is_ascii_digit())
                        .unwrap_or(false)
                } else {
                    false
                }
            });

            if !is_indexed {
                params.add(key, value.as_str());
                param_order.push(key.to_string());
            }
        }

        UnknownFields {
            params,
            binary: Vec::new(),
            param_order,
        }
    }

    /// Create from remaining binary bytes.
    pub fn from_remaining_binary(data: Vec<u8>) -> Self {
        UnknownFields {
            params: ParameterCollection::new(),
            binary: data,
            param_order: Vec::new(),
        }
    }

    /// Merge unknown parameters back into a parameter collection.
    ///
    /// Parameters are written in their original order to preserve
    /// file structure as much as possible. Parameters that already
    /// exist in the collection are NOT overwritten, so that struct
    /// fields (written by `append_to_params`) take priority over
    /// stale values captured in unknown_params during loading.
    pub fn merge_into_params(&self, params: &mut ParameterCollection) {
        for key in &self.param_order {
            if let Some(value) = self.params.get(key) {
                if !params.contains(key) {
                    params.add(key, value.as_str());
                }
            }
        }
    }

    /// Get the unknown binary data.
    pub fn binary_data(&self) -> &[u8] {
        &self.binary
    }

    /// Get mutable access to the unknown binary data.
    pub fn binary_data_mut(&mut self) -> &mut Vec<u8> {
        &mut self.binary
    }

    /// Check if there are any unknown fields.
    pub fn is_empty(&self) -> bool {
        self.params.is_empty() && self.binary.is_empty()
    }

    /// Get count of unknown parameters.
    pub fn param_count(&self) -> usize {
        self.param_order.len()
    }

    /// Get count of unknown binary bytes.
    pub fn binary_count(&self) -> usize {
        self.binary.len()
    }

    /// Get list of unknown parameter names (for debugging/logging).
    pub fn unknown_param_names(&self) -> &[String] {
        &self.param_order
    }

    /// Get a specific unknown parameter value.
    pub fn get_param(&self, key: &str) -> Option<String> {
        self.params.get(key).map(|v| v.as_str().to_string())
    }

    /// Set or add an unknown parameter.
    pub fn set_param(&mut self, key: &str, value: &str) {
        if self.params.get(key).is_none() {
            self.param_order.push(key.to_string());
        }
        self.params.add(key, value);
    }

    /// Remove an unknown parameter.
    pub fn remove_param(&mut self, key: &str) -> bool {
        if self.params.get(key).is_some() {
            self.param_order.retain(|k| k != key);
            // Note: ParameterCollection doesn't have remove, so we'd need to rebuild
            // For now, we just remove from order tracking
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_remaining_params() {
        let mut original = ParameterCollection::new();
        original.add("RECORD", "1");
        original.add("NAME", "Test");
        original.add("UNKNOWN1", "foo");
        original.add("UNKNOWN2", "bar");

        let known = &["RECORD", "NAME"];
        let unknown = UnknownFields::from_remaining_params(&original, known);

        assert_eq!(unknown.param_count(), 2);
        assert_eq!(unknown.get_param("UNKNOWN1"), Some("foo".to_string()));
        assert_eq!(unknown.get_param("UNKNOWN2"), Some("bar".to_string()));
    }

    #[test]
    fn test_merge_into_params() {
        let mut unknown = UnknownFields::new();
        unknown.set_param("EXTRA1", "value1");
        unknown.set_param("EXTRA2", "value2");

        let mut params = ParameterCollection::new();
        params.add("RECORD", "1");

        unknown.merge_into_params(&mut params);

        assert!(params.get("EXTRA1").is_some());
        assert!(params.get("EXTRA2").is_some());
    }
}
