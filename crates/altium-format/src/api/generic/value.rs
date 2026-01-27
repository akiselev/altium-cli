//! Dynamic value type for Altium data.

use crate::types::{Color, Coord, Layer, ParameterValue};

/// Dynamic value type for Altium data, similar to `serde_json::Value`.
///
/// This enum represents any value that can appear in an Altium record,
/// enabling dynamic access without compile-time type knowledge.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Value {
    /// Null or missing value
    #[default]
    Null,
    /// Boolean value
    Bool(bool),
    /// Integer value
    Int(i64),
    /// Floating-point value
    Float(f64),
    /// String value
    String(String),
    /// Coordinate value (preserves fractional precision)
    Coord(Coord),
    /// Color value (Win32 COLORREF)
    Color(Color),
    /// Layer value
    Layer(Layer),
    /// List of values (comma-separated in params)
    List(Vec<Value>),
    /// Raw binary data
    Binary(Vec<u8>),
}

impl Value {
    // --- Type checking ---

    /// Returns true if this value is null.
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Returns true if this value is a boolean.
    pub fn is_bool(&self) -> bool {
        matches!(self, Value::Bool(_))
    }

    /// Returns true if this value is an integer.
    pub fn is_int(&self) -> bool {
        matches!(self, Value::Int(_))
    }

    /// Returns true if this value is a float.
    pub fn is_float(&self) -> bool {
        matches!(self, Value::Float(_))
    }

    /// Returns true if this value is a string.
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    /// Returns true if this value is a coordinate.
    pub fn is_coord(&self) -> bool {
        matches!(self, Value::Coord(_))
    }

    /// Returns true if this value is a color.
    pub fn is_color(&self) -> bool {
        matches!(self, Value::Color(_))
    }

    /// Returns true if this value is a layer.
    pub fn is_layer(&self) -> bool {
        matches!(self, Value::Layer(_))
    }

    /// Returns true if this value is a list.
    pub fn is_list(&self) -> bool {
        matches!(self, Value::List(_))
    }

    /// Returns true if this value is binary data.
    pub fn is_binary(&self) -> bool {
        matches!(self, Value::Binary(_))
    }

    // --- Conversion with Option ---

    /// Returns the boolean value, if this is a Bool.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns the integer value, if this is an Int.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Returns the float value, if this is a Float.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Int(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Returns the string value, if this is a String.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the coordinate value, if this is a Coord.
    pub fn as_coord(&self) -> Option<Coord> {
        match self {
            Value::Coord(c) => Some(*c),
            _ => None,
        }
    }

    /// Returns the color value, if this is a Color.
    pub fn as_color(&self) -> Option<Color> {
        match self {
            Value::Color(c) => Some(*c),
            _ => None,
        }
    }

    /// Returns the layer value, if this is a Layer.
    pub fn as_layer(&self) -> Option<Layer> {
        match self {
            Value::Layer(l) => Some(*l),
            _ => None,
        }
    }

    /// Returns the list value, if this is a List.
    pub fn as_list(&self) -> Option<&[Value]> {
        match self {
            Value::List(l) => Some(l),
            _ => None,
        }
    }

    /// Returns the binary data, if this is Binary.
    pub fn as_binary(&self) -> Option<&[u8]> {
        match self {
            Value::Binary(b) => Some(b),
            _ => None,
        }
    }

    // --- Conversion with defaults ---

    /// Returns the boolean value, or a default.
    pub fn as_bool_or(&self, default: bool) -> bool {
        self.as_bool().unwrap_or(default)
    }

    /// Returns the integer value, or a default.
    pub fn as_int_or(&self, default: i64) -> i64 {
        self.as_int().unwrap_or(default)
    }

    /// Returns the float value, or a default.
    pub fn as_float_or(&self, default: f64) -> f64 {
        self.as_float().unwrap_or(default)
    }

    /// Returns the string value, or a default.
    pub fn as_str_or<'a>(&'a self, default: &'a str) -> &'a str {
        self.as_str().unwrap_or(default)
    }

    /// Returns the coordinate value, or a default.
    pub fn as_coord_or(&self, default: Coord) -> Coord {
        self.as_coord().unwrap_or(default)
    }

    // --- Construction from ParameterValue ---

    /// Creates a Value from a ParameterValue, inferring the type.
    pub fn from_param_value(pv: &ParameterValue) -> Self {
        let s = pv.as_str();

        // Try boolean
        match s.to_uppercase().as_str() {
            "T" | "TRUE" => return Value::Bool(true),
            "F" | "FALSE" => return Value::Bool(false),
            _ => {}
        }

        // Try integer
        if let Ok(i) = s.parse::<i64>() {
            return Value::Int(i);
        }

        // Try float
        if let Ok(f) = s.parse::<f64>() {
            return Value::Float(f);
        }

        // Try coordinate (ends with unit)
        if s.ends_with("mil") || s.ends_with("mm") || s.ends_with("in") {
            if let Ok(c) = pv.as_coord() {
                return Value::Coord(c);
            }
        }

        // Default to string
        Value::String(s.to_string())
    }

    /// Returns a human-readable type name.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Coord(_) => "coord",
            Value::Color(_) => "color",
            Value::Layer(_) => "layer",
            Value::List(_) => "list",
            Value::Binary(_) => "binary",
        }
    }
}

// --- From implementations ---

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::Int(v as i64)
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::Int(v)
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Float(v)
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::String(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::String(v.to_string())
    }
}

impl From<Coord> for Value {
    fn from(v: Coord) -> Self {
        Value::Coord(v)
    }
}

impl From<Color> for Value {
    fn from(v: Color) -> Self {
        Value::Color(v)
    }
}

impl From<Layer> for Value {
    fn from(v: Layer) -> Self {
        Value::Layer(v)
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Self {
        Value::List(v.into_iter().map(Into::into).collect())
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{}", if *b { "T" } else { "F" }),
            Value::Int(i) => write!(f, "{}", i),
            Value::Float(v) => write!(f, "{}", v),
            Value::String(s) => write!(f, "{}", s),
            Value::Coord(c) => write!(f, "{}mil", c.to_mils()),
            Value::Color(c) => write!(f, "{}", c.to_win32()),
            Value::Layer(l) => write!(f, "{}", l.0),
            Value::List(l) => {
                let items: Vec<String> = l.iter().map(|v| v.to_string()).collect();
                write!(f, "{}", items.join(","))
            }
            Value::Binary(b) => write!(f, "<{} bytes>", b.len()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_type_checks() {
        assert!(Value::Null.is_null());
        assert!(Value::Bool(true).is_bool());
        assert!(Value::Int(42).is_int());
        assert!(Value::String("test".into()).is_string());
    }

    #[test]
    fn test_value_conversions() {
        let v = Value::Int(42);
        assert_eq!(v.as_int(), Some(42));
        assert_eq!(v.as_float(), Some(42.0));
        assert_eq!(v.as_str(), None);
    }

    #[test]
    fn test_value_from() {
        let v: Value = 42i32.into();
        assert_eq!(v, Value::Int(42));

        let v: Value = "test".into();
        assert_eq!(v, Value::String("test".into()));
    }
}
