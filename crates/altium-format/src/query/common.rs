//! Shared types and utilities for query systems.
//!
//! This module contains common definitions used by both the record selector
//! system and the SchQL system, reducing code duplication.
//!
//! # V2 Migration Note
//! This module uses v1 types (PinElectricalType).
//! TODO: Migrate to v2 types when SchDocV2 provides equivalent functionality.

#![allow(deprecated)]

use super::pattern::Pattern;
use crate::records::sch::PinElectricalType;

/// Comparison operators for property/attribute filters.
///
/// Used by both the record selector `[prop op value]` syntax and
/// SchQL `[attr op value]` syntax.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterOp {
    /// `[attr]` - Has attribute (exists check)
    Exists,
    /// `=` - Exact match (case-insensitive for strings)
    Equals,
    /// `!=` - Not equal
    NotEquals,
    /// `~=` - Word match (matches word in space-separated list)
    WordMatch,
    /// `^=` - Starts with
    StartsWith,
    /// `$=` - Ends with
    EndsWith,
    /// `*=` - Contains substring
    Contains,
    /// `>` - Greater than (numeric)
    GreaterThan,
    /// `<` - Less than (numeric)
    LessThan,
    /// `>=` - Greater or equal (numeric)
    GreaterOrEqual,
    /// `<=` - Less or equal (numeric)
    LessOrEqual,
}

impl FilterOp {
    /// Parse operator from string representation.
    pub fn try_parse(s: &str) -> Option<Self> {
        match s {
            "=" => Some(Self::Equals),
            "!=" => Some(Self::NotEquals),
            "~=" => Some(Self::WordMatch),
            "^=" => Some(Self::StartsWith),
            "$=" => Some(Self::EndsWith),
            "*=" => Some(Self::Contains),
            ">" => Some(Self::GreaterThan),
            "<" => Some(Self::LessThan),
            ">=" => Some(Self::GreaterOrEqual),
            "<=" => Some(Self::LessOrEqual),
            _ => None,
        }
    }

    /// Get the string representation of this operator.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Exists => "",
            Self::Equals => "=",
            Self::NotEquals => "!=",
            Self::WordMatch => "~=",
            Self::StartsWith => "^=",
            Self::EndsWith => "$=",
            Self::Contains => "*=",
            Self::GreaterThan => ">",
            Self::LessThan => "<",
            Self::GreaterOrEqual => ">=",
            Self::LessOrEqual => "<=",
        }
    }
}

/// Value types for filter comparisons.
#[derive(Debug, Clone)]
pub enum FilterValue {
    /// String value
    String(String),
    /// Numeric value (stored as f64 for flexibility)
    Number(f64),
    /// Boolean value
    Bool(bool),
    /// Glob pattern for wildcard matching
    Pattern(Pattern),
}

impl FilterValue {
    /// Create a string value.
    pub fn string(s: impl Into<String>) -> Self {
        Self::String(s.into())
    }

    /// Create a numeric value.
    pub fn number(n: f64) -> Self {
        Self::Number(n)
    }

    /// Create a boolean value.
    pub fn bool(b: bool) -> Self {
        Self::Bool(b)
    }

    /// Try to get as string reference.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    /// Try to get as number.
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            Self::String(s) => s.parse().ok(),
            _ => None,
        }
    }
}

/// Compare a string value against a filter using the specified operator.
///
/// This is the shared comparison logic used by both query systems.
pub fn compare_filter(
    actual: Option<&str>,
    op: FilterOp,
    expected: &FilterValue,
    case_insensitive: bool,
) -> bool {
    match op {
        FilterOp::Exists => actual.is_some(),

        FilterOp::Equals => {
            let expected_str = match expected {
                FilterValue::String(s) => s.as_str(),
                FilterValue::Number(n) => return compare_numeric(actual, FilterOp::Equals, *n),
                FilterValue::Bool(b) => return compare_bool(actual, *b),
                FilterValue::Pattern(p) => return actual.is_some_and(|v| p.matches(v)),
            };
            actual.is_some_and(|v| {
                if case_insensitive {
                    v.eq_ignore_ascii_case(expected_str)
                } else {
                    v == expected_str
                }
            })
        }

        FilterOp::NotEquals => {
            let expected_str = match expected {
                FilterValue::String(s) => s.as_str(),
                FilterValue::Number(n) => return !compare_numeric(actual, FilterOp::Equals, *n),
                FilterValue::Bool(b) => return !compare_bool(actual, *b),
                FilterValue::Pattern(p) => return actual.is_none_or(|v| !p.matches(v)),
            };
            actual.is_none_or(|v| {
                if case_insensitive {
                    !v.eq_ignore_ascii_case(expected_str)
                } else {
                    v != expected_str
                }
            })
        }

        FilterOp::WordMatch => {
            let expected_str = match expected {
                FilterValue::String(s) => s.as_str(),
                _ => return false,
            };
            actual.is_some_and(|v| {
                v.split_whitespace().any(|word| {
                    if case_insensitive {
                        word.eq_ignore_ascii_case(expected_str)
                    } else {
                        word == expected_str
                    }
                })
            })
        }

        FilterOp::StartsWith => {
            let expected_str = match expected {
                FilterValue::String(s) => s.as_str(),
                _ => return false,
            };
            actual.is_some_and(|v| {
                if case_insensitive {
                    v.to_lowercase().starts_with(&expected_str.to_lowercase())
                } else {
                    v.starts_with(expected_str)
                }
            })
        }

        FilterOp::EndsWith => {
            let expected_str = match expected {
                FilterValue::String(s) => s.as_str(),
                _ => return false,
            };
            actual.is_some_and(|v| {
                if case_insensitive {
                    v.to_lowercase().ends_with(&expected_str.to_lowercase())
                } else {
                    v.ends_with(expected_str)
                }
            })
        }

        FilterOp::Contains => {
            let expected_str = match expected {
                FilterValue::String(s) => s.as_str(),
                _ => return false,
            };
            actual.is_some_and(|v| {
                if case_insensitive {
                    v.to_lowercase().contains(&expected_str.to_lowercase())
                } else {
                    v.contains(expected_str)
                }
            })
        }

        FilterOp::GreaterThan => {
            let n = expected.as_number().unwrap_or(0.0);
            compare_numeric(actual, FilterOp::GreaterThan, n)
        }

        FilterOp::LessThan => {
            let n = expected.as_number().unwrap_or(0.0);
            compare_numeric(actual, FilterOp::LessThan, n)
        }

        FilterOp::GreaterOrEqual => {
            let n = expected.as_number().unwrap_or(0.0);
            compare_numeric(actual, FilterOp::GreaterOrEqual, n)
        }

        FilterOp::LessOrEqual => {
            let n = expected.as_number().unwrap_or(0.0);
            compare_numeric(actual, FilterOp::LessOrEqual, n)
        }
    }
}

/// Compare a string value numerically.
fn compare_numeric(actual: Option<&str>, op: FilterOp, expected: f64) -> bool {
    let actual_num = actual.and_then(|v| v.parse::<f64>().ok());
    match actual_num {
        Some(n) => match op {
            FilterOp::Equals => (n - expected).abs() < 0.001,
            FilterOp::NotEquals => (n - expected).abs() >= 0.001,
            FilterOp::GreaterThan => n > expected,
            FilterOp::LessThan => n < expected,
            FilterOp::GreaterOrEqual => n >= expected,
            FilterOp::LessOrEqual => n <= expected,
            _ => false,
        },
        None => false,
    }
}

/// Compare a string value as boolean.
fn compare_bool(actual: Option<&str>, expected: bool) -> bool {
    let actual_bool =
        actual.map(|v| v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes") || v == "1");
    actual_bool == Some(expected)
}

/// Electrical type for pins (shared between systems).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ElectricalType {
    Input,
    Output,
    InputOutput,
    Passive,
    Power,
    OpenCollector,
    OpenEmitter,
    HiZ,
    Unknown,
}

impl ElectricalType {
    /// Parse from string representation.
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "input" | "in" => Self::Input,
            "output" | "out" => Self::Output,
            "io" | "inputoutput" | "input/output" | "bidirectional" | "bidir" => Self::InputOutput,
            "passive" | "pass" => Self::Passive,
            "power" | "pwr" => Self::Power,
            "opencollector" | "open collector" | "oc" => Self::OpenCollector,
            "openemitter" | "open emitter" | "oe" => Self::OpenEmitter,
            "hiz" | "hi-z" | "high-z" | "tristate" | "tri-state" => Self::HiZ,
            _ => Self::Unknown,
        }
    }

    /// Convert from Altium's PinElectricalType.
    pub fn from_pin_electrical(pe: PinElectricalType) -> Self {
        match pe {
            PinElectricalType::Input => Self::Input,
            PinElectricalType::Output => Self::Output,
            PinElectricalType::InputOutput => Self::InputOutput,
            PinElectricalType::Passive => Self::Passive,
            PinElectricalType::Power => Self::Power,
            PinElectricalType::OpenCollector => Self::OpenCollector,
            PinElectricalType::OpenEmitter => Self::OpenEmitter,
            PinElectricalType::HiZ => Self::HiZ,
        }
    }

    /// Get string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Input => "Input",
            Self::Output => "Output",
            Self::InputOutput => "Bidirectional",
            Self::Passive => "Passive",
            Self::Power => "Power",
            Self::OpenCollector => "OpenCollector",
            Self::OpenEmitter => "OpenEmitter",
            Self::HiZ => "HiZ",
            Self::Unknown => "Unknown",
        }
    }
}

impl std::fmt::Display for ElectricalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Electrical filter pseudo-selector (shared between systems).
///
/// These represent the electrical state filters that can be applied to pins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElectricalFilter {
    /// Pin is connected to a net
    Connected,
    /// Pin is not connected (floating)
    Unconnected,
    /// Input pin
    Input,
    /// Output pin
    Output,
    /// Bidirectional pin
    Bidirectional,
    /// Power pin
    Power,
    /// Ground connection
    Ground,
    /// Passive pin
    Passive,
    /// Open collector pin
    OpenCollector,
    /// Open emitter pin
    OpenEmitter,
    /// High-impedance pin
    HiZ,
}

impl ElectricalFilter {
    /// Parse from string.
    pub fn try_parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "connected" | "conn" => Some(Self::Connected),
            "unconnected" | "unconn" | "floating" | "nc" => Some(Self::Unconnected),
            "input" | "in" => Some(Self::Input),
            "output" | "out" => Some(Self::Output),
            "bidirectional" | "bidir" | "inout" => Some(Self::Bidirectional),
            "power" | "pwr" => Some(Self::Power),
            "ground" | "gnd" => Some(Self::Ground),
            "passive" | "pass" => Some(Self::Passive),
            "opencollector" | "oc" => Some(Self::OpenCollector),
            "openemitter" | "oe" => Some(Self::OpenEmitter),
            "hiz" | "tristate" => Some(Self::HiZ),
            _ => None,
        }
    }

    /// Check if an electrical type matches this filter.
    pub fn matches_type(&self, electrical_type: ElectricalType) -> bool {
        match self {
            Self::Input => electrical_type == ElectricalType::Input,
            Self::Output => electrical_type == ElectricalType::Output,
            Self::Bidirectional => electrical_type == ElectricalType::InputOutput,
            Self::Power => electrical_type == ElectricalType::Power,
            Self::Passive => electrical_type == ElectricalType::Passive,
            Self::OpenCollector => electrical_type == ElectricalType::OpenCollector,
            Self::OpenEmitter => electrical_type == ElectricalType::OpenEmitter,
            Self::HiZ => electrical_type == ElectricalType::HiZ,
            // Connected/Unconnected/Ground are not electrical types
            _ => false,
        }
    }
}

/// Visibility filter (shared between systems).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisibilityFilter {
    /// Element is visible (not hidden)
    Visible,
    /// Element is hidden
    Hidden,
}

impl VisibilityFilter {
    /// Parse from string.
    pub fn try_parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "visible" => Some(Self::Visible),
            "hidden" => Some(Self::Hidden),
            _ => None,
        }
    }

    /// Check if a hidden state matches this filter.
    pub fn matches(&self, is_hidden: bool) -> bool {
        match self {
            Self::Visible => !is_hidden,
            Self::Hidden => is_hidden,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_op_try_parse() {
        assert_eq!(FilterOp::try_parse("="), Some(FilterOp::Equals));
        assert_eq!(FilterOp::try_parse("!="), Some(FilterOp::NotEquals));
        assert_eq!(FilterOp::try_parse("^="), Some(FilterOp::StartsWith));
        assert_eq!(FilterOp::try_parse("$="), Some(FilterOp::EndsWith));
        assert_eq!(FilterOp::try_parse("*="), Some(FilterOp::Contains));
        assert_eq!(FilterOp::try_parse(">"), Some(FilterOp::GreaterThan));
        assert_eq!(FilterOp::try_parse("<"), Some(FilterOp::LessThan));
        assert_eq!(FilterOp::try_parse(">="), Some(FilterOp::GreaterOrEqual));
        assert_eq!(FilterOp::try_parse("<="), Some(FilterOp::LessOrEqual));
        assert_eq!(FilterOp::try_parse("??"), None);
    }

    #[test]
    fn test_compare_filter_equals() {
        let expected = FilterValue::string("hello");
        assert!(compare_filter(
            Some("hello"),
            FilterOp::Equals,
            &expected,
            false
        ));
        assert!(compare_filter(
            Some("HELLO"),
            FilterOp::Equals,
            &expected,
            true
        ));
        assert!(!compare_filter(
            Some("HELLO"),
            FilterOp::Equals,
            &expected,
            false
        ));
        assert!(!compare_filter(None, FilterOp::Equals, &expected, false));
    }

    #[test]
    fn test_compare_filter_exists() {
        let expected = FilterValue::string("");
        assert!(compare_filter(
            Some("anything"),
            FilterOp::Exists,
            &expected,
            false
        ));
        assert!(compare_filter(Some(""), FilterOp::Exists, &expected, false));
        assert!(!compare_filter(None, FilterOp::Exists, &expected, false));
    }

    #[test]
    fn test_compare_filter_contains() {
        let expected = FilterValue::string("test");
        assert!(compare_filter(
            Some("this is a test"),
            FilterOp::Contains,
            &expected,
            false
        ));
        assert!(compare_filter(
            Some("TEST value"),
            FilterOp::Contains,
            &expected,
            true
        ));
        assert!(!compare_filter(
            Some("no match"),
            FilterOp::Contains,
            &expected,
            false
        ));
    }

    #[test]
    fn test_compare_filter_numeric() {
        let expected = FilterValue::number(10.0);
        assert!(compare_filter(
            Some("15"),
            FilterOp::GreaterThan,
            &expected,
            false
        ));
        assert!(compare_filter(
            Some("5"),
            FilterOp::LessThan,
            &expected,
            false
        ));
        assert!(compare_filter(
            Some("10"),
            FilterOp::GreaterOrEqual,
            &expected,
            false
        ));
        assert!(!compare_filter(
            Some("5"),
            FilterOp::GreaterThan,
            &expected,
            false
        ));
    }

    #[test]
    fn test_electrical_type_parsing() {
        assert_eq!(ElectricalType::parse("Input"), ElectricalType::Input);
        assert_eq!(ElectricalType::parse("output"), ElectricalType::Output);
        assert_eq!(ElectricalType::parse("BIDIR"), ElectricalType::InputOutput);
        assert_eq!(ElectricalType::parse("power"), ElectricalType::Power);
    }

    #[test]
    fn test_electrical_filter_matches() {
        assert!(ElectricalFilter::Input.matches_type(ElectricalType::Input));
        assert!(!ElectricalFilter::Input.matches_type(ElectricalType::Output));
        assert!(ElectricalFilter::Bidirectional.matches_type(ElectricalType::InputOutput));
    }

    #[test]
    fn test_visibility_filter() {
        assert!(VisibilityFilter::Visible.matches(false));
        assert!(!VisibilityFilter::Visible.matches(true));
        assert!(VisibilityFilter::Hidden.matches(true));
        assert!(!VisibilityFilter::Hidden.matches(false));
    }
}
