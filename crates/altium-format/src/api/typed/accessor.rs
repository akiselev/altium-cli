//! TypedAccessor for strongly-typed record access.

use std::collections::HashSet;
use std::ops::{Deref, DerefMut};

use crate::api::generic::GenericRecord;
use crate::error::Result;
use crate::traits::{FromParams, ToParams};
use crate::types::ParameterCollection;

/// Accessor for typed records with non-destructive editing support.
///
/// Wraps a typed record `T` along with its `GenericRecord` backing,
/// enabling access to both typed fields and unknown fields while
/// preserving the ability to serialize back without data loss.
#[derive(Debug, Clone)]
pub struct TypedAccessor<T> {
    /// The typed record
    record: T,
    /// Generic backing for order preservation and unknown fields
    backing: GenericRecord,
    /// Tracks which typed fields were accessed mutably
    modified_typed_fields: HashSet<String>,
}

impl<T: FromParams + ToParams> TypedAccessor<T> {
    /// Creates a TypedAccessor from a ParameterCollection.
    pub fn from_params(params: &ParameterCollection) -> Result<Self> {
        let record = T::from_params(params)?;
        let backing = GenericRecord::from_params(params);

        Ok(TypedAccessor {
            record,
            backing,
            modified_typed_fields: HashSet::new(),
        })
    }

    /// Creates a TypedAccessor from a GenericRecord.
    pub fn from_generic(generic: GenericRecord) -> Result<Self> {
        let params = generic.to_params();
        let record = T::from_params(&params)?;

        Ok(TypedAccessor {
            record,
            backing: generic,
            modified_typed_fields: HashSet::new(),
        })
    }

    /// Returns a reference to the typed record.
    pub fn get(&self) -> &T {
        &self.record
    }

    /// Returns a mutable reference to the typed record.
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.record
    }

    /// Returns the backing GenericRecord (for unknown fields).
    pub fn backing(&self) -> &GenericRecord {
        &self.backing
    }

    /// Returns mutable access to the backing GenericRecord.
    pub fn backing_mut(&mut self) -> &mut GenericRecord {
        &mut self.backing
    }

    /// Returns true if the record was modified.
    pub fn is_modified(&self) -> bool {
        !self.modified_typed_fields.is_empty() || self.backing.is_modified()
    }

    /// Converts back to a ParameterCollection, preserving order.
    ///
    /// The typed record's fields are merged with the backing's preserved
    /// order and unknown fields.
    pub fn to_params(&self) -> ParameterCollection {
        // Start with backing's order
        let mut params = self.backing.to_params();

        // Overlay typed record's values
        let mut typed_params = ParameterCollection::new();
        self.record.append_to_params(&mut typed_params);

        // Merge typed values into the ordered params
        for (key, value) in typed_params.iter() {
            params.add(key, value.as_str());
        }

        params
    }

    /// Converts back to a GenericRecord.
    pub fn to_generic(&self) -> GenericRecord {
        GenericRecord::from_params(&self.to_params())
    }

    /// Consumes the accessor and returns the typed record.
    pub fn into_inner(self) -> T {
        self.record
    }
}

impl<T> Deref for TypedAccessor<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.record
    }
}

impl<T> DerefMut for TypedAccessor<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.record
    }
}

#[cfg(test)]
mod tests {
    // TypedAccessor tests require concrete types that implement FromParams/ToParams.
    // See records/sch/tests.rs for integration tests with SchComponent, SchPin, etc.
}
