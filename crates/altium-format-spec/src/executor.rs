//! Executor: applies spec models directly to Altium documents.
//!
//! Currently unimplemented — the LowOps pipeline has been removed.
//! A high-level API on the document types will replace this.

use altium_format::{PcbLib, SchLib};

use crate::eval::{SpecError, SpecErrorCode};
use crate::model::{PcbLibSpec, SchLibSpec};

/// Apply a SchLib spec directly to a document.
pub fn apply_spec_schlib(
    _spec: &SchLibSpec,
    _doc: &mut SchLib,
) -> Result<(), SpecError> {
    Err(SpecError::no_span(
        SpecErrorCode::TypeMismatch,
        "spec executor removed; high-level API pending",
    ))
}

/// Apply a PcbLib spec directly to a document.
pub fn apply_spec_pcblib(
    _spec: &PcbLibSpec,
    _lib: &mut PcbLib,
) -> Result<(), SpecError> {
    Err(SpecError::no_span(
        SpecErrorCode::TypeMismatch,
        "spec executor removed; high-level API pending",
    ))
}
