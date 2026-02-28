use crate::ast::{CompareOp, FilterValue};
use crate::diagnostic::Unit;

/// Runtime value extracted from an entity field.
#[derive(Debug, Clone, PartialEq)]
pub enum QueryValue {
    String(String),
    Integer(i64),
    Float(f64),
    Bool(bool),
    /// Internal coordinate value (10,000 units = 1 mil).
    Coord(i32),
    /// RGB color.
    Color(u8, u8, u8),
    /// The field is absent or not applicable.
    Null,
}

impl QueryValue {
    /// Test whether this value matches a filter value using the given operator.
    ///
    /// Returns `false` if `self` is `Null` (missing fields never match).
    /// Type coercion is applied where sensible (e.g., Coord compared to Dim).
    pub fn matches(&self, op: CompareOp, filter: &FilterValue) -> bool {
        match self {
            QueryValue::Null => false,
            QueryValue::String(s) => match_string(s, op, filter),
            QueryValue::Integer(n) => match_integer(*n, op, filter),
            QueryValue::Float(f) => match_float(*f, op, filter),
            QueryValue::Bool(b) => match_bool(*b, op, filter),
            QueryValue::Coord(c) => match_coord(*c, op, filter),
            QueryValue::Color(r, g, b) => match_color(*r, *g, *b, op, filter),
        }
    }
}

fn match_string(s: &str, op: CompareOp, filter: &FilterValue) -> bool {
    let filter_str = filter_as_str(filter);
    match op {
        CompareOp::Eq => s.eq_ignore_ascii_case(&filter_str),
        CompareOp::NotEq => !s.eq_ignore_ascii_case(&filter_str),
        CompareOp::Contains => s.to_ascii_lowercase().contains(&filter_str.to_ascii_lowercase()),
        CompareOp::StartsWith => s.to_ascii_lowercase().starts_with(&filter_str.to_ascii_lowercase()),
        CompareOp::EndsWith => s.to_ascii_lowercase().ends_with(&filter_str.to_ascii_lowercase()),
        CompareOp::WordMatch => {
            let lower = filter_str.to_ascii_lowercase();
            s.split_whitespace()
                .any(|word| word.to_ascii_lowercase() == lower)
        }
        // Numeric ops on strings: try to parse as number
        CompareOp::Gt | CompareOp::Lt | CompareOp::Gte | CompareOp::Lte => false,
    }
}

fn match_integer(n: i64, op: CompareOp, filter: &FilterValue) -> bool {
    let rhs = match filter {
        FilterValue::Integer(i) => *i,
        FilterValue::Float(f) => *f as i64,
        FilterValue::Ident(s) | FilterValue::String(s) => match s.parse::<i64>() {
            Ok(v) => v,
            Err(_) => return op == CompareOp::NotEq,
        },
        FilterValue::Dim(val, unit) => unit_to_internal(*val, *unit) as i64,
        _ => return op == CompareOp::NotEq,
    };
    compare_ord(n, rhs, op)
}

fn match_float(f: f64, op: CompareOp, filter: &FilterValue) -> bool {
    let rhs = match filter {
        FilterValue::Float(v) => *v,
        FilterValue::Integer(i) => *i as f64,
        FilterValue::Ident(s) | FilterValue::String(s) => match s.parse::<f64>() {
            Ok(v) => v,
            Err(_) => return op == CompareOp::NotEq,
        },
        _ => return op == CompareOp::NotEq,
    };
    compare_float(f, rhs, op)
}

fn match_bool(b: bool, op: CompareOp, filter: &FilterValue) -> bool {
    let rhs = match filter {
        FilterValue::Bool(v) => *v,
        FilterValue::Ident(s) | FilterValue::String(s) => match s.to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" => true,
            "false" | "0" | "no" => false,
            _ => return op == CompareOp::NotEq,
        },
        FilterValue::Integer(n) => *n != 0,
        _ => return op == CompareOp::NotEq,
    };
    match op {
        CompareOp::Eq => b == rhs,
        CompareOp::NotEq => b != rhs,
        _ => false,
    }
}

fn match_coord(c: i32, op: CompareOp, filter: &FilterValue) -> bool {
    let rhs = match filter {
        FilterValue::Dim(val, unit) => unit_to_internal(*val, *unit),
        FilterValue::Integer(n) => {
            // Bare integers treated as mils
            (*n as f64 * 10_000.0).round() as i32
        }
        FilterValue::Float(f) => {
            // Bare floats treated as mils
            (*f * 10_000.0).round() as i32
        }
        FilterValue::Ident(s) | FilterValue::String(s) => match s.parse::<f64>() {
            Ok(v) => (v * 10_000.0).round() as i32,
            Err(_) => return op == CompareOp::NotEq,
        },
        _ => return op == CompareOp::NotEq,
    };
    compare_ord(c, rhs, op)
}

fn match_color(r: u8, g: u8, b: u8, op: CompareOp, filter: &FilterValue) -> bool {
    let rhs = filter_as_str(filter);
    let self_str = format!("#{r:02X}{g:02X}{b:02X}");
    match op {
        CompareOp::Eq => self_str.eq_ignore_ascii_case(&rhs),
        CompareOp::NotEq => !self_str.eq_ignore_ascii_case(&rhs),
        _ => false,
    }
}

fn compare_ord<T: Ord>(lhs: T, rhs: T, op: CompareOp) -> bool {
    match op {
        CompareOp::Eq => lhs == rhs,
        CompareOp::NotEq => lhs != rhs,
        CompareOp::Gt => lhs > rhs,
        CompareOp::Lt => lhs < rhs,
        CompareOp::Gte => lhs >= rhs,
        CompareOp::Lte => lhs <= rhs,
        _ => false,
    }
}

fn compare_float(lhs: f64, rhs: f64, op: CompareOp) -> bool {
    match op {
        CompareOp::Eq => (lhs - rhs).abs() < f64::EPSILON,
        CompareOp::NotEq => (lhs - rhs).abs() >= f64::EPSILON,
        CompareOp::Gt => lhs > rhs,
        CompareOp::Lt => lhs < rhs,
        CompareOp::Gte => lhs >= rhs,
        CompareOp::Lte => lhs <= rhs,
        _ => false,
    }
}

/// Convert a filter value to its string representation for comparison.
fn filter_as_str(filter: &FilterValue) -> String {
    match filter {
        FilterValue::String(s) | FilterValue::Ident(s) | FilterValue::Regex(s) => s.clone(),
        FilterValue::Integer(n) => n.to_string(),
        FilterValue::Float(f) => f.to_string(),
        FilterValue::Bool(b) => b.to_string(),
        FilterValue::Dim(val, unit) => format!("{val}{unit}"),
    }
}

/// Convert a dimensional value to internal coordinate units.
///
/// Uses the same conversion factors as the spec crate:
/// - 1 mil = 10,000 internal units
/// - 1 mm ≈ 393,701 internal units
/// - 1 inch = 10,000,000 internal units
pub fn unit_to_internal(value: f64, unit: Unit) -> i32 {
    match unit {
        Unit::Mil => (value * 10_000.0).round() as i32,
        Unit::Mm => (value * 393_701.0).round() as i32,
        Unit::Inch => (value * 10_000_000.0).round() as i32,
    }
}

/// Check if a string matches a regex pattern.
pub fn regex_matches(value: &str, pattern: &str) -> bool {
    match regex::Regex::new(pattern) {
        Ok(re) => re.is_match(value),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_eq() {
        let v = QueryValue::String("Hello".into());
        assert!(v.matches(CompareOp::Eq, &FilterValue::String("hello".into())));
        assert!(v.matches(CompareOp::Eq, &FilterValue::Ident("HELLO".into())));
        assert!(!v.matches(CompareOp::Eq, &FilterValue::String("world".into())));
    }

    #[test]
    fn test_string_contains() {
        let v = QueryValue::String("Hello World".into());
        assert!(v.matches(CompareOp::Contains, &FilterValue::String("world".into())));
        assert!(!v.matches(CompareOp::Contains, &FilterValue::String("xyz".into())));
    }

    #[test]
    fn test_string_starts_with() {
        let v = QueryValue::String("Hello World".into());
        assert!(v.matches(CompareOp::StartsWith, &FilterValue::String("hello".into())));
    }

    #[test]
    fn test_string_word_match() {
        let v = QueryValue::String("do not place".into());
        assert!(v.matches(CompareOp::WordMatch, &FilterValue::String("not".into())));
        assert!(!v.matches(CompareOp::WordMatch, &FilterValue::String("no".into())));
    }

    #[test]
    fn test_integer_comparisons() {
        let v = QueryValue::Integer(42);
        assert!(v.matches(CompareOp::Eq, &FilterValue::Integer(42)));
        assert!(v.matches(CompareOp::Gt, &FilterValue::Integer(10)));
        assert!(v.matches(CompareOp::Lte, &FilterValue::Integer(42)));
        assert!(!v.matches(CompareOp::Lt, &FilterValue::Integer(42)));
    }

    #[test]
    fn test_bool_comparisons() {
        let v = QueryValue::Bool(true);
        assert!(v.matches(CompareOp::Eq, &FilterValue::Bool(true)));
        assert!(!v.matches(CompareOp::Eq, &FilterValue::Bool(false)));
        assert!(v.matches(CompareOp::Eq, &FilterValue::Ident("true".into())));
    }

    #[test]
    fn test_coord_with_dim() {
        // 100mil = 1,000,000 internal units
        let v = QueryValue::Coord(1_000_000);
        assert!(v.matches(CompareOp::Eq, &FilterValue::Dim(100.0, Unit::Mil)));
        assert!(v.matches(CompareOp::Gte, &FilterValue::Dim(50.0, Unit::Mil)));
    }

    #[test]
    fn test_null_never_matches() {
        let v = QueryValue::Null;
        assert!(!v.matches(CompareOp::Eq, &FilterValue::String("anything".into())));
        assert!(!v.matches(CompareOp::NotEq, &FilterValue::String("anything".into())));
    }

    #[test]
    fn test_unit_to_internal_conversions() {
        assert_eq!(unit_to_internal(1.0, Unit::Mil), 10_000);
        assert_eq!(unit_to_internal(1.0, Unit::Mm), 393_701);
        assert_eq!(unit_to_internal(1.0, Unit::Inch), 10_000_000);
    }

    #[test]
    fn test_color_eq() {
        let v = QueryValue::Color(255, 0, 0);
        assert!(v.matches(CompareOp::Eq, &FilterValue::String("#FF0000".into())));
        assert!(!v.matches(CompareOp::Eq, &FilterValue::String("#00FF00".into())));
    }
}
