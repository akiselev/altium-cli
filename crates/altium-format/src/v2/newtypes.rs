//! Domain newtypes for Altium format values.
//!
//! These types wrap `String` to provide type safety and domain-specific
//! methods for common Altium identifiers: designators, library references,
//! net names, unique IDs, descriptions, and pin names.

use serde::{Deserialize, Serialize};

use crate::v2::parameters::ParameterCollection;
use crate::v2::traits::ParamCodec;

// ---------------------------------------------------------------------------
// Macro: impl_string_newtype!
// ---------------------------------------------------------------------------

/// Generates common boilerplate for string-wrapper newtypes:
/// `Deref<Target=str>`, `Display`, `From<&str>`, `From<String>`, and `ParamCodec`.
macro_rules! impl_string_newtype {
    ($name:ident) => {
        impl std::ops::Deref for $name {
            type Target = str;
            fn deref(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_string())
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }

        impl ParamCodec for $name {
            fn read(params: &ParameterCollection, key: &str) -> Option<Self> {
                params.get(key).map(|v| Self(v.as_str().to_string()))
            }

            fn write(&self, params: &mut ParameterCollection, key: &str) {
                params.add(key, &self.0);
            }
        }
    };
}

// ---------------------------------------------------------------------------
// Designator
// ---------------------------------------------------------------------------

/// Component designator such as "R1", "U10", or template "U?".
///
/// Designators consist of a letter prefix followed by either a numeric
/// suffix or a "?" placeholder (template designator).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Designator(String);

impl_string_newtype!(Designator);

impl Designator {
    /// Creates a new `Designator` from anything that converts to `String`.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns the inner string as a `&str`.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the alphabetic prefix of the designator.
    ///
    /// The prefix is all leading characters up to (but not including) the
    /// first ASCII digit or "?" character.
    ///
    /// # Examples
    /// - `"R1"` -> `"R"`
    /// - `"U10"` -> `"U"`
    /// - `"IC3"` -> `"IC"`
    /// - `"R?"` -> `"R"`
    pub fn prefix(&self) -> &str {
        let end = self
            .0
            .find(|c: char| c.is_ascii_digit() || c == '?')
            .unwrap_or(self.0.len());
        &self.0[..end]
    }

    /// Returns `true` if this is a template designator (contains "?" suffix).
    ///
    /// # Examples
    /// - `"U?"` -> `true`
    /// - `"U1"` -> `false`
    pub fn is_template(&self) -> bool {
        self.0.ends_with('?')
    }

    /// Returns the numeric portion of the designator, if present.
    ///
    /// # Examples
    /// - `"R1"` -> `Some(1)`
    /// - `"U10"` -> `Some(10)`
    /// - `"R?"` -> `None`
    pub fn number(&self) -> Option<u32> {
        let start = self
            .0
            .find(|c: char| c.is_ascii_digit())?;
        self.0[start..].parse().ok()
    }

    /// Sets the numeric portion of the designator, replacing any existing
    /// number or "?" suffix.
    ///
    /// # Examples
    /// - `"R1".set_number(5)` -> `"R5"`
    /// - `"R?".set_number(3)` -> `"R3"`
    pub fn set_number(&mut self, n: u32) {
        let prefix = self.prefix().to_string();
        self.0 = format!("{}{}", prefix, n);
    }

    /// Increments the numeric portion by 1. If the designator is a template
    /// or has no number, this is a no-op.
    ///
    /// # Examples
    /// - `"R1"` -> `"R2"`
    /// - `"U10"` -> `"U11"`
    pub fn increment(&mut self) {
        if let Some(n) = self.number() {
            self.set_number(n + 1);
        }
    }

    /// Resolves a template designator with the given number.
    ///
    /// Returns a new `Designator` with the "?" replaced by the number.
    /// If the designator is not a template, returns a clone with the
    /// number set.
    ///
    /// # Examples
    /// - `"U?"` + 3 -> `"U3"`
    /// - `"R?"` + 10 -> `"R10"`
    pub fn resolve(&self, n: u32) -> Designator {
        let prefix = self.prefix().to_string();
        Designator(format!("{}{}", prefix, n))
    }
}

// ---------------------------------------------------------------------------
// LibReference
// ---------------------------------------------------------------------------

/// Library component reference identifier.
///
/// Used to look up components in schematic libraries.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LibReference(String);

impl_string_newtype!(LibReference);

impl LibReference {
    /// Creates a new `LibReference`.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns a normalized (uppercase, trimmed) version of the reference.
    pub fn normalize(&self) -> String {
        self.0.trim().to_uppercase()
    }

    /// Returns `true` if this reference matches the given glob-like pattern.
    ///
    /// Supports `*` as a wildcard that matches any sequence of characters.
    /// Comparison is case-insensitive.
    ///
    /// # Examples
    /// - `"Resistor"` matches `"Res*"`
    /// - `"Cap_100nF"` matches `"Cap_*"`
    /// - `"LED_Red"` does not match `"Res*"`
    pub fn matches_pattern(&self, pattern: &str) -> bool {
        let self_upper = self.0.to_uppercase();
        let pattern_upper = pattern.to_uppercase();

        if !pattern_upper.contains('*') {
            return self_upper == pattern_upper;
        }

        let parts: Vec<&str> = pattern_upper.split('*').collect();

        // Single trailing wildcard (most common case: "Prefix*")
        if parts.len() == 2 && parts[1].is_empty() {
            return self_upper.starts_with(parts[0]);
        }

        // General glob matching with multiple wildcards
        let mut pos = 0;
        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                continue;
            }
            if let Some(found) = self_upper[pos..].find(part) {
                // First segment must match at the start
                if i == 0 && found != 0 {
                    return false;
                }
                pos += found + part.len();
            } else {
                return false;
            }
        }

        // If pattern doesn't end with '*', remainder must be consumed
        if !pattern_upper.ends_with('*') {
            return pos == self_upper.len();
        }

        true
    }
}

// ---------------------------------------------------------------------------
// NetName
// ---------------------------------------------------------------------------

/// Electrical net name in a schematic or PCB.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NetName(String);

impl_string_newtype!(NetName);

/// Common power net prefixes in Altium schematics.
const POWER_NET_PREFIXES: &[&str] = &[
    "VCC", "VDD", "VEE", "VSS", "GND", "DGND", "AGND", "PGND",
    "+3V3", "+5V", "+12V", "-12V", "+3.3V", "+1.8V", "+2.5V",
    "3V3", "5V", "12V",
];

impl NetName {
    /// Creates a new `NetName`.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns `true` if this looks like a power net.
    ///
    /// Checks against common power supply net name prefixes (VCC, GND, +5V, etc.).
    pub fn is_power_net(&self) -> bool {
        let upper = self.0.to_uppercase();
        POWER_NET_PREFIXES
            .iter()
            .any(|prefix| upper.starts_with(prefix))
    }

    /// Returns the prefix portion of a hierarchical net name.
    ///
    /// If the net name contains a backslash separator, returns everything
    /// before the last separator. Otherwise returns the full name.
    ///
    /// # Examples
    /// - `"Sheet1\\NET1"` -> `"Sheet1"`
    /// - `"GND"` -> `"GND"`
    pub fn prefix(&self) -> &str {
        match self.0.rfind('\\') {
            Some(pos) => &self.0[..pos],
            None => &self.0,
        }
    }
}

// ---------------------------------------------------------------------------
// UniqueId
// ---------------------------------------------------------------------------

/// Altium unique identifier (8-character alphanumeric string).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UniqueId(String);

impl_string_newtype!(UniqueId);

impl UniqueId {
    /// Creates a new `UniqueId`.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Generates a new random 8-character alphanumeric unique ID.
    ///
    /// Uses UUID v4 as a source of randomness and takes the first 8
    /// characters of its hex representation.
    pub fn generate() -> Self {
        let uuid = uuid::Uuid::new_v4();
        let hex = uuid.as_simple().to_string();
        // Take the first 8 characters (all hex chars are alphanumeric)
        Self(hex[..8].to_uppercase())
    }

    /// Returns `true` if this is a valid Altium unique ID.
    ///
    /// A valid ID is exactly 8 characters long and contains only
    /// ASCII alphanumeric characters.
    pub fn is_valid(&self) -> bool {
        self.0.len() == 8 && self.0.chars().all(|c| c.is_ascii_alphanumeric())
    }
}

// ---------------------------------------------------------------------------
// Description
// ---------------------------------------------------------------------------

/// Component or object description string.
///
/// Thin wrapper providing type safety with no additional domain logic.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Description(String);

impl_string_newtype!(Description);

impl Description {
    /// Creates a new `Description`.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

// ---------------------------------------------------------------------------
// PinName
// ---------------------------------------------------------------------------

/// Pin name with support for Altium's overbar (inversion) syntax.
///
/// In Altium, a leading `~` indicates that the pin name should be displayed
/// with an overbar, signifying an active-low signal.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PinName(String);

impl_string_newtype!(PinName);

impl PinName {
    /// Creates a new `PinName`.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns `true` if this pin name is inverted (starts with `~`).
    ///
    /// # Examples
    /// - `"~RESET"` -> `true`
    /// - `"CLK"` -> `false`
    pub fn is_inverted(&self) -> bool {
        self.0.starts_with('~')
    }

    /// Returns the display text, stripping the leading `~` if present.
    ///
    /// # Examples
    /// - `"~RESET"` -> `"RESET"`
    /// - `"CLK"` -> `"CLK"`
    pub fn display_text(&self) -> &str {
        self.0.strip_prefix('~').unwrap_or(&self.0)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Designator tests --

    #[test]
    fn designator_prefix() {
        assert_eq!(Designator::new("R1").prefix(), "R");
        assert_eq!(Designator::new("U10").prefix(), "U");
        assert_eq!(Designator::new("IC3").prefix(), "IC");
        assert_eq!(Designator::new("R?").prefix(), "R");
        assert_eq!(Designator::new("LED1").prefix(), "LED");
    }

    #[test]
    fn designator_number() {
        assert_eq!(Designator::new("R1").number(), Some(1));
        assert_eq!(Designator::new("U10").number(), Some(10));
        assert_eq!(Designator::new("R?").number(), None);
        assert_eq!(Designator::new("IC").number(), None);
    }

    #[test]
    fn designator_is_template() {
        assert!(Designator::new("U?").is_template());
        assert!(Designator::new("R?").is_template());
        assert!(!Designator::new("U1").is_template());
        assert!(!Designator::new("R10").is_template());
    }

    #[test]
    fn designator_increment() {
        let mut d = Designator::new("R1");
        d.increment();
        assert_eq!(d.as_str(), "R2");

        let mut d = Designator::new("U10");
        d.increment();
        assert_eq!(d.as_str(), "U11");

        // Template designator: increment is a no-op
        let mut d = Designator::new("R?");
        d.increment();
        assert_eq!(d.as_str(), "R?");
    }

    #[test]
    fn designator_resolve() {
        assert_eq!(Designator::new("U?").resolve(3).as_str(), "U3");
        assert_eq!(Designator::new("R?").resolve(10).as_str(), "R10");
        assert_eq!(Designator::new("IC?").resolve(1).as_str(), "IC1");
    }

    #[test]
    fn designator_set_number() {
        let mut d = Designator::new("R1");
        d.set_number(5);
        assert_eq!(d.as_str(), "R5");

        let mut d = Designator::new("R?");
        d.set_number(3);
        assert_eq!(d.as_str(), "R3");
    }

    #[test]
    fn designator_display_and_deref() {
        let d = Designator::new("R1");
        assert_eq!(format!("{}", d), "R1");
        assert_eq!(d.len(), 2); // via Deref<Target=str>
    }

    #[test]
    fn designator_from_conversions() {
        let d1: Designator = "R1".into();
        let d2: Designator = String::from("R1").into();
        assert_eq!(d1, d2);
    }

    // -- LibReference tests --

    #[test]
    fn lib_reference_normalize() {
        let r = LibReference::new("  Resistor  ");
        assert_eq!(r.normalize(), "RESISTOR");
    }

    #[test]
    fn lib_reference_matches_pattern() {
        let r = LibReference::new("Resistor");
        assert!(r.matches_pattern("Res*"));
        assert!(r.matches_pattern("RESISTOR"));
        assert!(r.matches_pattern("resistor"));
        assert!(!r.matches_pattern("Cap*"));

        let c = LibReference::new("Cap_100nF");
        assert!(c.matches_pattern("Cap_*"));
        assert!(c.matches_pattern("*100*"));
        assert!(!c.matches_pattern("Res*"));
    }

    // -- NetName tests --

    #[test]
    fn net_name_is_power_net() {
        assert!(NetName::new("VCC").is_power_net());
        assert!(NetName::new("GND").is_power_net());
        assert!(NetName::new("+5V").is_power_net());
        assert!(NetName::new("+3V3").is_power_net());
        assert!(NetName::new("vcc").is_power_net());
        assert!(!NetName::new("SDA").is_power_net());
        assert!(!NetName::new("NET1").is_power_net());
    }

    #[test]
    fn net_name_prefix() {
        assert_eq!(NetName::new("Sheet1\\NET1").prefix(), "Sheet1");
        assert_eq!(NetName::new("GND").prefix(), "GND");
    }

    // -- PinName tests --

    #[test]
    fn pin_name_inverted() {
        assert!(PinName::new("~RESET").is_inverted());
        assert!(PinName::new("~CS").is_inverted());
        assert!(!PinName::new("CLK").is_inverted());
        assert!(!PinName::new("SDA").is_inverted());
    }

    #[test]
    fn pin_name_display_text() {
        assert_eq!(PinName::new("~RESET").display_text(), "RESET");
        assert_eq!(PinName::new("CLK").display_text(), "CLK");
    }

    // -- UniqueId tests --

    #[test]
    fn unique_id_generate() {
        let id = UniqueId::generate();
        assert!(id.is_valid(), "generated ID '{}' should be valid", id);

        // Two generated IDs should be different
        let id2 = UniqueId::generate();
        assert_ne!(id, id2);
    }

    #[test]
    fn unique_id_is_valid() {
        assert!(UniqueId::new("ABCD1234").is_valid());
        assert!(UniqueId::new("abcd1234").is_valid());
        assert!(!UniqueId::new("ABC").is_valid()); // too short
        assert!(!UniqueId::new("ABCD12345").is_valid()); // too long
        assert!(!UniqueId::new("ABCD-123").is_valid()); // invalid char
    }

    // -- Description tests --

    #[test]
    fn description_basic() {
        let d = Description::new("100k Resistor");
        assert_eq!(&*d, "100k Resistor");
        assert_eq!(format!("{}", d), "100k Resistor");
    }
}
