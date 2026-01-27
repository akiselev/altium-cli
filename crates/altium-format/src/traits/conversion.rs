//! Value conversion traits for parameter serialization.

use crate::error::{AltiumError, Result};
use crate::types::ParameterValue;

/// Convert from a parameter value to a Rust type.
///
/// This trait is used by the derive macro to convert individual parameter
/// values to their corresponding Rust types.
pub trait FromParamValue: Sized {
    /// Parse this type from a parameter value.
    fn from_param_value(value: &ParameterValue) -> Result<Self>;

    /// Default value to use if the parameter is missing.
    fn default_value() -> Option<Self> {
        None
    }
}

/// Convert from a Rust type to a parameter value string.
pub trait ToParamValue {
    /// Convert this value to a parameter string.
    fn to_param_value(&self) -> String;

    /// Whether this value should be skipped if it equals the default.
    fn should_skip_default(&self) -> bool {
        false
    }
}

/// Convert from a list parameter (comma-separated values).
pub trait FromParamList: Sized {
    /// Parse this type from a comma-separated parameter value.
    fn from_param_list(value: &ParameterValue) -> Result<Self>;
}

/// Convert to a list parameter (comma-separated values).
pub trait ToParamList {
    /// Convert this value to a comma-separated string.
    fn to_param_list(&self) -> String;
}

// ============================================================================
// Basic type implementations
// ============================================================================

impl FromParamValue for i32 {
    fn from_param_value(value: &ParameterValue) -> Result<Self> {
        value
            .as_int()
            .map_err(|e| AltiumError::InvalidParameter(e.to_string()))
    }

    fn default_value() -> Option<Self> {
        Some(0)
    }
}

impl ToParamValue for i32 {
    fn to_param_value(&self) -> String {
        self.to_string()
    }

    fn should_skip_default(&self) -> bool {
        *self == 0
    }
}

impl FromParamValue for i64 {
    fn from_param_value(value: &ParameterValue) -> Result<Self> {
        value
            .as_str()
            .parse()
            .map_err(|e: std::num::ParseIntError| AltiumError::InvalidParameter(e.to_string()))
    }

    fn default_value() -> Option<Self> {
        Some(0)
    }
}

impl ToParamValue for i64 {
    fn to_param_value(&self) -> String {
        self.to_string()
    }
}

impl FromParamValue for u32 {
    fn from_param_value(value: &ParameterValue) -> Result<Self> {
        value
            .as_str()
            .parse()
            .map_err(|e: std::num::ParseIntError| AltiumError::InvalidParameter(e.to_string()))
    }

    fn default_value() -> Option<Self> {
        Some(0)
    }
}

impl ToParamValue for u32 {
    fn to_param_value(&self) -> String {
        self.to_string()
    }
}

impl FromParamValue for f64 {
    fn from_param_value(value: &ParameterValue) -> Result<Self> {
        value
            .as_double()
            .map_err(|e| AltiumError::InvalidParameter(e.to_string()))
    }

    fn default_value() -> Option<Self> {
        Some(0.0)
    }
}

impl ToParamValue for f64 {
    fn to_param_value(&self) -> String {
        self.to_string()
    }
}

impl FromParamValue for bool {
    fn from_param_value(value: &ParameterValue) -> Result<Self> {
        value
            .as_bool()
            .map_err(|e| AltiumError::InvalidParameter(e.to_string()))
    }

    fn default_value() -> Option<Self> {
        Some(false)
    }
}

impl ToParamValue for bool {
    fn to_param_value(&self) -> String {
        if *self {
            "T".to_string()
        } else {
            "F".to_string()
        }
    }

    fn should_skip_default(&self) -> bool {
        !*self
    }
}

impl FromParamValue for String {
    fn from_param_value(value: &ParameterValue) -> Result<Self> {
        Ok(value.as_str().to_string())
    }

    fn default_value() -> Option<Self> {
        Some(String::new())
    }
}

impl ToParamValue for String {
    fn to_param_value(&self) -> String {
        self.clone()
    }

    fn should_skip_default(&self) -> bool {
        self.is_empty()
    }
}

impl<T: FromParamValue> FromParamValue for Option<T> {
    fn from_param_value(value: &ParameterValue) -> Result<Self> {
        Ok(Some(T::from_param_value(value)?))
    }

    fn default_value() -> Option<Self> {
        Some(None)
    }
}

impl<T: ToParamValue> ToParamValue for Option<T> {
    fn to_param_value(&self) -> String {
        match self {
            Some(v) => v.to_param_value(),
            None => String::new(),
        }
    }

    fn should_skip_default(&self) -> bool {
        self.is_none()
    }
}

// ============================================================================
// List implementations
// ============================================================================

impl<T: FromParamValue> FromParamList for Vec<T> {
    fn from_param_list(value: &ParameterValue) -> Result<Self> {
        let s = value.as_str();
        if s.is_empty() {
            return Ok(Vec::new());
        }

        s.split(',')
            .map(|part| {
                let trimmed = part.trim();
                // Create a temporary ParameterValue for each part (level 0)
                let temp_value = ParameterValue::new(trimmed.to_string(), 0);
                T::from_param_value(&temp_value)
            })
            .collect()
    }
}

impl<T: ToParamValue> ToParamList for Vec<T> {
    fn to_param_list(&self) -> String {
        self.iter()
            .map(|v| v.to_param_value())
            .collect::<Vec<_>>()
            .join(",")
    }
}
