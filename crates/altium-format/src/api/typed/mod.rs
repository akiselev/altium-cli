//! Layer 3: Typed deserialization with derive macro support.
//!
//! Provides strongly-typed access to Altium records using the existing
//! `FromParams`/`ToParams` traits and derive macros.

mod accessor;
mod editor;

pub use accessor::TypedAccessor;
pub use editor::EditTransaction;
