//! Errors from the spec-to-IR compiler.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum IrCompileError {
    #[error("no boards defined in spec")]
    NoBoardsDefined,
    #[error("duplicate designator: {0}")]
    DuplicateDesignator(String),
    #[error("unknown net: {0}")]
    UnknownNet(String),
    #[error("unknown layer: {0}")]
    UnknownLayer(String),
    #[error("unknown rule kind: {0}")]
    UnknownRuleKind(String),
    #[error("missing board outline")]
    MissingBoardOutline,
    #[error("invalid scope expression: {0}")]
    InvalidScope(String),
    #[error("unsupported pad shape: {0}")]
    UnsupportedPadShape(String),
    #[error("invalid value for property '{0}': '{1}'")]
    InvalidPropertyValue(String, String),
    #[error("unknown footprint: ${0}.{1} not found in imported .sym files")]
    UnknownFootprint(String, String),
}
