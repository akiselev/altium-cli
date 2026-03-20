//! Intermediate representation for PCB placement and routing.
//!
//! Extracts a simplified, mm-based IR from an `altium_format::PcbDocBoard`,
//! suitable for placement solvers, routers, DRC, and visualisation.

pub mod board;
pub mod component;
pub mod component_body;
pub mod copper;
pub mod dimension;
pub mod extract;
pub mod handles;
pub mod layer_stack;
pub mod net;
pub mod polygon;
pub mod region;
pub mod rule;
pub mod spec_bridge;
pub mod text;
pub mod types;

pub use board::{IrBoardGeometry, IrKeepoutZone};
pub use component::{IrComponent, IrComponentPad, PadShapeInfo, PadShapeKind};
pub use component_body::IrComponentBody;
pub use copper::{FreeCopperGeometry, IrArc, IrFill, IrTrack, IrVia};
pub use extract::PcbIr;
pub use handles::{
    ComponentBodyId, ComponentId, DimensionId, IdMap, LayerId, NetId, PadId, PolygonId, RegionId,
    RuleId, TextId,
};
pub use layer_stack::{IrCopperLayer, IrLayerStack, PreferredDirection};
pub use net::{IrNet, IrNetPin};
pub use polygon::IrPolygon;
pub use region::{IrRegion, IrRegionKind};
pub use rule::{IrDesignRule, IrRuleParams};
pub use spec_bridge::load_ir_from_spec;
pub use text::IrText;
pub use types::{BoardSide, BoundingBoxMm, PointMm};

/// Errors from IR extraction.
#[derive(Debug, thiserror::Error)]
pub enum IrError {
    #[error("PcbDoc has no board outline")]
    NoBoardOutline,

    #[error("extraction failed: {0}")]
    ExtractionError(String),
}

pub type Result<T> = std::result::Result<T, IrError>;
