//! V2 Altium format API — ID-handle architecture.

// Foundation
pub mod backing_store;
pub mod coord;
pub mod traits;
pub mod newtypes;
pub mod binary_helpers;

// ID types and store
pub mod ids;
pub mod store;
pub mod handles;
pub mod semantic_ids;

// PCB-specific types (enums, etc.)
pub mod pcb;

// Record types — populated by macro-generated types
pub mod records;

// Document types
pub mod documents;

// Query language
pub mod query;

// Templates & builders
pub mod templates;
pub mod builders;

// ParameterCollection (from v1, self-contained)
pub mod parameters;
