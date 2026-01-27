//! Parameter-based serialization traits for schematic records.

use crate::error::Result;
use crate::types::{ParameterCollection, UnknownFields};

/// Import a type from a parameter collection.
///
/// This trait is automatically implemented by the `AltiumRecord` and `AltiumBase`
/// derive macros for parameter-based record types.
pub trait FromParams: Sized {
    /// Parse this type from a parameter collection.
    fn from_params(params: &ParameterCollection) -> Result<Self>;

    /// Parse this type and collect unknown parameters for non-destructive editing.
    fn from_params_preserving(params: &ParameterCollection) -> Result<(Self, UnknownFields)> {
        let value = Self::from_params(params)?;
        Ok((value, UnknownFields::default()))
    }
}

/// Export a type to a parameter collection.
///
/// This trait is automatically implemented by the `AltiumRecord` and `AltiumBase`
/// derive macros for parameter-based record types.
pub trait ToParams {
    /// Export this type to a new parameter collection.
    fn to_params(&self) -> ParameterCollection {
        let mut params = ParameterCollection::new();
        self.append_to_params(&mut params);
        params
    }

    /// Append this type's parameters to an existing collection.
    ///
    /// This is used for composition - base types append their fields first,
    /// then derived types append their own fields.
    fn append_to_params(&self, params: &mut ParameterCollection);
}

// Blanket implementations for Option<T>
impl<T: FromParams> FromParams for Option<T> {
    fn from_params(params: &ParameterCollection) -> Result<Self> {
        // For Option, we try to parse but return None on any failure
        match T::from_params(params) {
            Ok(v) => Ok(Some(v)),
            Err(_) => Ok(None),
        }
    }
}

impl<T: ToParams> ToParams for Option<T> {
    fn append_to_params(&self, params: &mut ParameterCollection) {
        if let Some(v) = self {
            v.append_to_params(params);
        }
    }
}
