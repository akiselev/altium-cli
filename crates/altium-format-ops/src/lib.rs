pub use altium_format::AltiumFormatError;

pub mod intlib_ops;
pub mod ops;
pub mod parser;
pub mod pcbdoc_ops;
pub mod pcblib_ops;
pub mod project_ops;
pub mod schdoc_ops;
pub mod schlib_ops;

pub use intlib_ops::IntLibOps;
pub use ops::{
    AddAliasOp, AddArcHighOp, AddBezierHighOp, AddComponentOp, AddEllipseHighOp,
    AddEllipticalArcHighOp, AddImageHighOp, AddLabelHighOp, AddLineHighOp, AddParameterOp,
    AddPieHighOp, AddPinOp, AddPolygonHighOp, AddPolylineHighOp, AddRectangleHighOp,
    AddRoundRectangleHighOp, AddTextFrameHighOp, ApplyReport, ApplySpec, EditComponentHighOp,
    EditRecordHighOp, HighOp, QueryComponentsHighOp, QueryHighOp, QueryPinsHighOp,
    QueryRecordsHighOp, Ref, RefExpr, RefRoot, RefStep, RemoveAliasOp, RemoveComponentOp,
    RemoveRecordsHighOp, Value, apply_schdoc, apply_schlib, parse_apply_spec_json,
    parse_apply_spec_yaml,
};
pub use pcbdoc_ops::PcbDocOps;
pub use pcblib_ops::PcbLibOps;
pub use project_ops::AltiumProjectOps;
pub use schdoc_ops::SchDocOps;
pub use schlib_ops::SchLibOps;

/// Version information extracted from an Altium document's file header.
#[derive(Debug, serde::Serialize)]
pub struct VersionInfo {
    /// On-disk header string identifying the file format (e.g. "Protel for Windows - Schematic
    /// Library Editor Binary File Version 5.0").
    pub header: String,
    /// Minor version number from the file header. Incremented as Altium Designer adds
    /// forward-compatibility features.
    pub minor_version: i32,
    /// Optional `FileVersionInfo` blob written by the saving application.
    pub file_version_info: Option<String>,
}

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
