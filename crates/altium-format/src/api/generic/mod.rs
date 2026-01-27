//! Layer 2: Generic record access without type knowledge.
//!
//! Provides dynamic access to Altium records similar to `serde_json::Value`,
//! allowing exploration and modification without knowing specific types.

mod binary_record;
mod container;
mod record;
mod value;

pub use binary_record::BinaryRecord;
pub use container::{BinaryContainer, ParamsContainer};
pub use record::GenericRecord;
pub use value::Value;
