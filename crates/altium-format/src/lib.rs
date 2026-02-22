// Unused until TrackedCfbDocument (Layer 2, Milestone 3) is implemented.
#[allow(dead_code)]
mod cfb_document;
pub mod document;
pub mod intlib;
pub mod pcbdoc;
pub mod pcblib;
pub mod project;
pub mod schdoc;
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
    #[error("Binary read past end: needed {needed} bytes at offset {offset}, only {available} remain")]
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
    #[error("Unknown binary code in schematic block: 0x{0:02X}")]
    UnknownBinaryCode(u8),
    #[error("Unknown parameters remaining: {keys:?}")]
    UnknownParams { keys: Vec<String> },
    #[error("Unexpected trailing data: {count} bytes remaining at offset {offset}")]
    UnexpectedTrailingData { offset: usize, count: usize },
}

pub type Result<T> = std::result::Result<T, AltiumFormatError>;
