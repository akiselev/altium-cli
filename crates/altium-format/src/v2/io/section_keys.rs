//! Section key generation ported from `SchDataComponentSectionKeyList.cs`.
//!
//! Section keys map component names to CFB storage paths. Names longer than
//! `maxKeyLength` (30 in Altium) are truncated with collision avoidance
//! using numeric suffixes (`_1`, `_2`, etc.).

use std::collections::{BTreeMap, BTreeSet};

/// Maps full component names to section keys for CFB storage paths.
///
/// Ported from C# `SchDataComponentSectionKeyList`.
#[derive(Clone, Debug, Default)]
pub struct SectionKeyList {
    /// Maps full name → (name, key) pair.
    name_key_map: BTreeMap<String, (String, String)>,
    /// Set of allocated keys for collision detection.
    key_set: BTreeSet<String>,
}

impl SectionKeyList {
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears all mappings.
    pub fn clear(&mut self) {
        self.name_key_map.clear();
        self.key_set.clear();
    }

    /// Adds a key mapping for a component name.
    ///
    /// From C#: Only processes names where `name.Length >= maxKeyLength`.
    /// The key is truncated to `max_key_length` with numeric suffix collision avoidance.
    ///
    /// C# also checks `text2.Length >= 30 && text2[30] == ' '` to avoid
    /// keys that happen to have a space at position 30.
    pub fn add_key(&mut self, name: &str, max_key_length: usize) {
        // C#: if (string.IsNullOrEmpty(name) || name.Length < maxKeyLength) return;
        if name.is_empty() || name.len() < max_key_length {
            return;
        }

        let mut base: String = name.chars().take(max_key_length).collect();
        let mut suffix = 1u32;
        let mut candidate = base.clone();

        // C#: while (keyList.ContainsKey(text2) || (text2.Length >= 30 && text2[30] == ' '))
        while self.key_set.contains(&candidate)
            || (candidate.len() >= 30
                && candidate
                    .chars()
                    .nth(30)
                    .map_or(false, |c| c == ' '))
        {
            let suffix_str = suffix.to_string();
            if base.len() + suffix_str.len() > max_key_length {
                base = name
                    .chars()
                    .take(max_key_length - suffix_str.len())
                    .collect();
            }
            candidate = format!("{}{}", base, suffix_str);
            suffix += 1;
        }

        self.key_set.insert(candidate.clone());
        self.name_key_map
            .entry(name.to_string())
            .or_insert_with(|| (name.to_string(), candidate));
    }

    /// Gets the section key for a component name.
    ///
    /// Returns the name itself if no mapping exists (short names don't need keys).
    pub fn get_key<'a>(&'a self, name: &'a str) -> &'a str {
        self.name_key_map
            .get(name)
            .map(|(_, key)| key.as_str())
            .unwrap_or(name)
    }

    /// Returns true if there are any key mappings.
    pub fn is_empty(&self) -> bool {
        self.name_key_map.is_empty()
    }

    /// Number of key mappings.
    pub fn len(&self) -> usize {
        self.name_key_map.len()
    }

    /// Iterator over (name, key) pairs in sorted order.
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
    fn short_name_not_added() {
        let mut keys = SectionKeyList::new();
        keys.add_key("ShortName", 31);
        assert!(keys.is_empty());
        assert_eq!(keys.get_key("ShortName"), "ShortName");
    }

    #[test]
    fn long_name_truncated() {
        let mut keys = SectionKeyList::new();
        let name = "A".repeat(40);
        keys.add_key(&name, 31);
        assert_eq!(keys.len(), 1);
        let key = keys.get_key(&name);
        assert_eq!(key.len(), 31);
        assert_eq!(key, &"A".repeat(31));
    }

    #[test]
    fn collision_avoidance() {
        let mut keys = SectionKeyList::new();
        let name1 = format!("{}X", "A".repeat(30));
        let name2 = format!("{}Y", "A".repeat(30));

        keys.add_key(&name1, 31);
        keys.add_key(&name2, 31);

        let key1 = keys.get_key(&name1);
        let key2 = keys.get_key(&name2);
        assert_ne!(key1, key2);
        assert!(key1.len() <= 31);
        assert!(key2.len() <= 31);
    }

    #[test]
    fn max_key_length_30() {
        // Test with Altium's actual maxKeyLength of 30
        let mut keys = SectionKeyList::new();
        let name = "B".repeat(35);
        keys.add_key(&name, 30);
        let key = keys.get_key(&name);
        assert_eq!(key.len(), 30);
        assert_eq!(key, &"B".repeat(30));
    }

    #[test]
    fn collision_with_suffix() {
        let mut keys = SectionKeyList::new();
        // Two names that truncate to the same 30-char prefix
        let name1 = format!("{}1", "C".repeat(30));
        let name2 = format!("{}2", "C".repeat(30));
        let name3 = format!("{}3", "C".repeat(30));

        keys.add_key(&name1, 30);
        keys.add_key(&name2, 30);
        keys.add_key(&name3, 30);

        let key1 = keys.get_key(&name1);
        let key2 = keys.get_key(&name2);
        let key3 = keys.get_key(&name3);

        // All keys must be unique and within length limit
        assert_ne!(key1, key2);
        assert_ne!(key2, key3);
        assert_ne!(key1, key3);
        assert!(key1.len() <= 30);
        assert!(key2.len() <= 30);
        assert!(key3.len() <= 30);
    }
}
