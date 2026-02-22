// altium-format-types: Type definitions, enums, and constants for Altium Designer file formats.
//
// This crate contains all shared and domain-specific type definitions used by altium-format.
// No parsing logic, no I/O - just pure type definitions.

pub mod coord;
pub mod color;
pub mod unique_id;
pub mod common;
pub mod sch;
pub mod pcb;
pub mod constants;

/// Error returned when a raw integer value does not correspond to any known enum variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidEnumValue {
    pub type_name: &'static str,
    pub value: i64,
}

impl std::fmt::Display for InvalidEnumValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid value {} for enum {}", self.value, self.type_name)
    }
}

impl std::error::Error for InvalidEnumValue {}

pub use coord::{BoundingBox, Coord, CoordPoint};
pub use color::Color;
pub use unique_id::{UniqueId, UniqueIdError};
pub use common::{ComponentKind, RotationBy90, TextAutoPosition, Unit};
pub use pcb::{
    BoardSide, CornerStyle, DielectricType, DimensionKind, PcbFileFormatVersion, PcbFlags,
    PcbObjectId, PadShape, PadShapeSubKind, PadStackMode, RuleKind, V6Layer, V7Layer,
};
pub use sch::{
    ConnectorKind, ConnectorState, HarnessBrush, HarnessCavityPartType,
    HarnessConnectionPointStyle, HarnessLengthUnit, HarnessShieldStyle, HarnessSpliceStyle,
    HarnessWireLengthType, LeftRightSide, NoErcSymbol, ParameterReadOnlyState, ParameterSetStyle,
    ParameterType, PinElectricalType, SchRecordType, SheetReferenceZoneStyle, SheetSymbolType,
    TextHorzAnchor, TextVertAnchor, VisibleGridStyle,
};
