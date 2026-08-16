//! # altium-spec-lang
//!
//! Foundational language types for the Altium spec toolchain. The complete
//! legacy semantic parser/compiler still lives in `altium-format-spec` during
//! migration, but synchronization no longer needs to depend on its CST internals:
//! this crate owns lossless source retention, source-node identity, explicit
//! field-absence semantics, and the structural authored-intent boundary.
//!
//! Dependency direction stays one-way:
//!
//! `altium-format-types -> altium-spec-lang -> altium-sync`.

pub mod intent;
pub mod source;

pub use intent::{AuthoredIntent, AuthoredResource, FieldIntent};
pub use source::{
    LosslessSpec, ResourceBlock, SourceError, SourceNodeId, SpecDomain, canonicalize_semantic_text,
};
