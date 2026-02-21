//! Error types for the Altium library.

use thiserror::Error;

/// Main error type for Altium file operations.
#[derive(Error, Debug)]
pub enum AltiumError {
    /// I/O error when reading/writing files.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Invalid parameter format.
    #[error("Invalid parameter format: {0}")]
    InvalidParameter(String),

    /// Invalid coordinate format.
    #[error("Invalid coordinate format: {0}")]
    InvalidCoordinate(String),

    /// Invalid unit format.
    #[error("Invalid unit: {0}")]
    InvalidUnit(String),

    /// Invalid layer.
    #[error("Invalid layer: {0}")]
    InvalidLayer(String),

    /// Invalid record type.
    #[error("Invalid record type: {0}")]
    InvalidRecord(String),

    /// Missing required data.
    #[error("Missing required data: {0}")]
    MissingData(String),

    /// Missing required parameter (key present in schema but absent from file).
    #[error("Missing parameter: {0}")]
    MissingParameter(String),

    /// Decompression error (zlib).
    #[error("Decompression error: {0}")]
    Decompression(String),

    /// Encoding error (Windows-1252 or UTF-8).
    #[error("Encoding error: {0}")]
    Encoding(String),

    /// Unexpected end of stream.
    #[error("Unexpected end of stream")]
    UnexpectedEof,

    /// Generic parse error.
    #[error("Parse error: {0}")]
    Parse(String),

    /// Validation error for invalid data values.
    #[error("Validation error: {0}")]
    Validation(String),

    /// Template application error.
    #[error("Template error: {0}")]
    Template(String),

    /// Query evaluation error.
    #[error("query error: {0}")]
    Query(String),

    /// No match found for a query.
    #[error("no match found: {0}")]
    NoMatch(String),

    /// Ambiguous match: multiple results where one was expected.
    #[error("ambiguous match: {0} matches found for query '{1}'")]
    AmbiguousMatch(usize, String),

    /// CFB (Compound File Binary) format error.
    #[error("CFB error: {0}")]
    Cfb(String),
}

/// Result type alias for Altium operations.
pub type Result<T> = std::result::Result<T, AltiumError>;
