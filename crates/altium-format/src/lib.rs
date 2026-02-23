#[allow(dead_code)]
mod binary_io;
#[allow(dead_code)]
mod block_stream;
#[allow(dead_code)]
mod board_config;
#[allow(dead_code)]
mod cfb_document;
#[cfg(test)]
mod derive_tests;
#[allow(dead_code)]
mod embedded_object;
#[allow(dead_code)]
mod param_collection;
#[allow(dead_code)]
mod param_value;
#[allow(dead_code)]
mod pcb_binary_stream;
#[allow(dead_code)]
mod pcb_file_header;
#[allow(dead_code)]
mod prefixed_param_stream;
#[allow(dead_code)]
mod sch_records;
#[allow(dead_code)]
mod tracked_cfb;
#[allow(dead_code)]
mod wide_strings_tlv;

#[allow(dead_code)]
pub mod document;
#[allow(dead_code)]
pub mod intlib;
#[allow(dead_code)]
pub mod pcbdoc;
#[allow(dead_code)]
pub mod pcblib;
#[allow(dead_code)]
pub mod project;
#[allow(dead_code)]
pub mod schdoc;
#[allow(dead_code)]
pub mod schlib;

pub use document::Document;
pub use intlib::IntLib;
pub use pcbdoc::PcbDoc;
pub use pcblib::PcbLib;
pub use project::AltiumProject;
pub use schdoc::SchDoc;
pub use schlib::SchLib;

#[derive(Debug, thiserror::Error)]
pub enum AltiumFormatError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    // Layer 1: CFB container errors
    #[error("CFB format error: {0}")]
    CfbError(String),
    #[error("Stream not found: {0}")]
    StreamNotFound(String),

    // Layer 2: Stream consumption tracking
    #[error("Unconsumed streams/storages in CFB container: {paths:?}")]
    UnconsumedStreams { paths: Vec<String> },

    // Layer 3: Block stream framing
    #[error("Invalid block header at offset {offset}: {detail}")]
    InvalidBlockHeader { offset: usize, detail: String },
    #[error("Record count mismatch in {section}: expected {expected}, got {actual}")]
    RecordCountMismatch {
        section: String,
        expected: usize,
        actual: usize,
    },

    // Layer 4: Structured data access
    #[error("Missing required parameter: {0}")]
    MissingParam(String),
    #[error("Decompression failed: {0}")]
    DecompressionError(String),
    #[error("Invalid parameter value for key '{key}': {detail}")]
    InvalidParamValue { key: String, detail: String },
    #[error(
        "Binary read past end: needed {needed} bytes at offset {offset}, only {available} remain"
    )]
    BinaryReadPastEnd {
        offset: usize,
        needed: usize,
        available: usize,
    },

    // Layer 4: Embedded object envelope
    #[error("Invalid embedded object: {0}")]
    InvalidEmbeddedObject(String),

    // Layer 5: Strict validation
    #[error("Unknown record type: {0}")]
    UnknownRecordType(i32),
    #[error("Unknown PCB object ID: {0}")]
    UnknownObjectId(u8),
    #[error("Invalid enum value: {0}")]
    InvalidEnumValue(#[from] altium_format_types::InvalidEnumValue),
    #[error("Unknown binary code in schematic block: 0x{0:02X}")]
    UnknownBinaryCode(u8),
    #[error("Unknown parameters remaining: {keys:?}")]
    UnknownParams { keys: Vec<String> },
    #[error("Sidecar pin index {index} out of range (component has {count} pins)")]
    InvalidPinIndex { index: usize, count: usize },
    #[error("Unexpected trailing data: {count} bytes remaining at offset {offset}")]
    UnexpectedTrailingData { offset: usize, count: usize },

    // Context wrapper for chaining location info (e.g. "parsing component 'X': ...")
    #[error("{context}: {source}")]
    WithContext {
        context: String,
        source: Box<AltiumFormatError>,
    },
}

pub type Result<T> = std::result::Result<T, AltiumFormatError>;

/// Extension trait for attaching location context to errors.
pub(crate) trait ResultExt<T> {
    fn context(self, msg: &str) -> Result<T>;
    fn with_context(self, f: impl FnOnce() -> String) -> Result<T>;
}

impl<T> ResultExt<T> for Result<T> {
    fn context(self, msg: &str) -> Result<T> {
        self.map_err(|e| AltiumFormatError::WithContext {
            context: msg.to_owned(),
            source: Box::new(e),
        })
    }

    fn with_context(self, f: impl FnOnce() -> String) -> Result<T> {
        self.map_err(|e| AltiumFormatError::WithContext {
            context: f(),
            source: Box::new(e),
        })
    }
}
