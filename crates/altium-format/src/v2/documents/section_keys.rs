//! Section key generation for CFB storage paths.
//!
//! In Altium SchLib files, component names can exceed the 31-character limit
//! for CFB storage names. This module generates shortened keys that are unique
//! within a file and maps them back to the original component names.

use std::collections::{BTreeMap, BTreeSet};

/// Maintains a mapping between component names and their CFB storage keys.
///
/// When a component name is too long for a CFB storage entry (which has a
/// 31-character limit), this generates a unique shortened key by truncating
/// and appending a numeric suffix if needed.
#[derive(Clone, Debug, Default)]
pub struct SectionKeyList {
    /// Maps original name -> (name, generated key).
    name_key_map: BTreeMap<String, (String, String)>,
    /// Set of all generated keys (for uniqueness checking).
    key_set: BTreeSet<String>,
}

impl SectionKeyList {
    /// Creates a new empty section key list.
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears all stored keys.
    pub fn clear(&mut self) {
        self.name_key_map.clear();
        self.key_set.clear();
    }

    /// Adds a key mapping for the given name, truncating to `max_key_length`
    /// if the name is too long. If a collision occurs, appends a numeric suffix.
    pub fn add_key(&mut self, name: &str, max_key_length: usize) {
        if name.is_empty() || name.len() < max_key_length {
            return;
        }
        let mut base: String = name.chars().take(max_key_length).collect();
        let mut suffix = 1u32;
        let mut candidate = base.clone();
        while self.key_set.contains(&candidate)
            || (candidate.len() >= 30
                && candidate.chars().nth(30).map_or(false, |c| c == ' '))
        {
            let suffix_str = suffix.to_string();
            if base.len() + suffix_str.len() > max_key_length {
                base = name.chars().take(max_key_length - suffix_str.len()).collect();
            }
            candidate = format!("{}{}", base, suffix_str);
            suffix += 1;
        }
        self.key_set.insert(candidate.clone());
        self.name_key_map
            .entry(name.to_string())
            .or_insert_with(|| (name.to_string(), candidate));
    }

    /// Inserts a pre-existing name-to-key mapping directly (used when reading
    /// section keys from a file, where we want to preserve the original keys).
    pub fn insert_mapping(&mut self, name: &str, key: &str) {
        self.key_set.insert(key.to_string());
        self.name_key_map
            .entry(name.to_string())
            .or_insert_with(|| (name.to_string(), key.to_string()));
    }

    /// Returns the shortened key for a given name, or the name itself if
    /// no key mapping exists (i.e., the name was short enough).
    pub fn get_key<'a>(&'a self, name: &'a str) -> &'a str {
        self.name_key_map
            .get(name)
            .map(|(_, key)| key.as_str())
            .unwrap_or(name)
    }

    /// Returns true if no key mappings have been added.
    pub fn is_empty(&self) -> bool {
        self.name_key_map.is_empty()
    }

    /// Returns the number of key mappings.
    pub fn len(&self) -> usize {
        self.name_key_map.len()
    }

    /// Returns an iterator over (name, key) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.name_key_map
            .values()
            .map(|(name, key)| (name.as_str(), key.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_names_not_added() {
        let mut keys = SectionKeyList::new();
        keys.add_key("short", 30);
        assert!(keys.is_empty());
        assert_eq!(keys.get_key("short"), "short");
    }

    #[test]
    fn long_names_truncated() {
        let mut keys = SectionKeyList::new();
        let long_name = "A".repeat(40);
        keys.add_key(&long_name, 30);
        assert_eq!(keys.len(), 1);
        let key = keys.get_key(&long_name);
        assert!(key.len() <= 30);
    }

    #[test]
    fn collision_adds_suffix() {
        let mut keys = SectionKeyList::new();
        // Two names that truncate to the same 30 chars
        let name1 = format!("{}X", "A".repeat(30));
        let name2 = format!("{}Y", "A".repeat(30));
        keys.add_key(&name1, 30);
        keys.add_key(&name2, 30);
        assert_eq!(keys.len(), 2);
        let key1 = keys.get_key(&name1);
        let key2 = keys.get_key(&name2);
        assert_ne!(key1, key2);
    }

    #[test]
    fn insert_mapping_preserves_keys() {
        let mut keys = SectionKeyList::new();
        keys.insert_mapping("MyComponent", "MyComp");
        assert_eq!(keys.get_key("MyComponent"), "MyComp");
        assert_eq!(keys.len(), 1);
    }
}
