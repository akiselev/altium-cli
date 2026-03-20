//! Intermediate representation for PCB placement and routing.
//!
//! Extracts a simplified, mm-based IR from an `altium_format::PcbDocBoard`,
//! suitable for placement solvers, routers, DRC, and visualisation.

pub mod board;
pub mod component;
pub mod copper;
pub mod extract;
pub mod handles;
pub mod layer_stack;
pub mod net;
pub mod polygon;
pub mod rule;
pub mod types;

pub use board::{IrBoardGeometry, IrKeepoutZone};
pub use component::{IrComponent, IrComponentPad, PadShapeInfo, PadShapeKind};
pub use copper::{FreeCopperGeometry, IrFill, IrTrack, IrVia};
pub use extract::PcbIr;
pub use handles::{ComponentId, IdMap, LayerId, NetId, PadId, PolygonId, RuleId};
pub use layer_stack::{IrCopperLayer, IrLayerStack, PreferredDirection};
pub use net::{IrNet, IrNetPin};
pub use polygon::IrPolygon;
pub use rule::{IrDesignRule, IrRuleParams};
pub use types::{BoardSide, BoundingBoxMm, PointMm};

/// Errors from IR extraction.
#[derive(Debug, thiserror::Error)]
pub enum IrError {
    #[error("PcbDoc has no board outline")]
    NoBoardOutline,

    #[error("extraction failed: {0}")]
    ExtractionError(String),

    #[error(transparent)]
    FormatError(#[from] altium_format::AltiumFormatError),
}

pub type Result<T> = std::result::Result<T, IrError>;
