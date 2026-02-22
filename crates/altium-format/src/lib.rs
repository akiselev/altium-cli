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
    #[error("Invalid parameter value: {0}")]
    InvalidParamValue(String),
    #[error("Unknown record type: {0}")]
    UnknownRecordType(i32),
    #[error("Binary parsing error: {0}")]
    BinaryParsingError(String),
}

pub type Result<T> = std::result::Result<T, AltiumFormatError>;
