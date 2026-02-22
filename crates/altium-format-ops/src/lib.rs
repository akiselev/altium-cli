pub use altium_format::AltiumFormatError;

pub mod intlib_ops;
pub mod pcbdoc_ops;
pub mod pcblib_ops;
pub mod project_ops;
pub mod schdoc_ops;
pub mod schlib_ops;

pub use intlib_ops::IntLibOps;
pub use pcbdoc_ops::PcbDocOps;
pub use pcblib_ops::PcbLibOps;
pub use project_ops::AltiumProjectOps;
pub use schdoc_ops::SchDocOps;
pub use schlib_ops::SchLibOps;

#[derive(Debug, thiserror::Error)]
pub enum AltiumOperationError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Altium format error: {0}")]
    AltiumFormat(#[from] AltiumFormatError),
    #[error("Unimplemented operation: {0}")]
    Unimplemented(String),
}

pub type Result<T> = std::result::Result<T, AltiumOperationError>;
