//! V2 Altium format API — backing-store architecture.

// Foundation (Phase 1)
pub mod backing_store;
pub mod coord;
pub mod traits;
pub mod newtypes;
pub mod binary_helpers;

// Record types (Phase 3) — populated by macro-generated types
pub mod records;

// View types (Phase 4)
pub mod views;

// Document types (Phase 4)
pub mod documents;

// Query language (Phase 5)
pub mod query;

// Templates & builders (Phase 6)
pub mod templates;
pub mod builders;

// CLI operations (Phase 7)
pub mod ops;

// ParameterCollection (from v1, self-contained)
pub mod parameters;
