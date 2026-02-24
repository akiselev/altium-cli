// altium-format-types: Type definitions, enums, and constants for Altium Designer file formats.
//
// This crate contains all shared and domain-specific type definitions used by altium-format.
// No parsing logic, no I/O - just pure type definitions.

pub mod color;
pub mod common;
pub mod constants;
pub mod coord;
pub mod pcb;
pub mod sch;
pub mod unique_id;

/// Error returned when a raw integer value does not correspond to any known enum variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidEnumValue {
    pub type_name: &'static str,
    pub value: i64,
}

impl std::fmt::Display for InvalidEnumValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid value {} for enum {}",
            self.value, self.type_name
        )
    }
}

impl std::error::Error for InvalidEnumValue {}

pub use color::Color;
pub use common::{ComponentKind, RotationBy90, TextAutoPosition, Unit};
pub use coord::{BoundingBox, Coord, CoordPoint};
pub use pcb::{
    BoardSide, CornerStyle, DielectricType, DimensionKind, HoleType, MaskExpansionMode, PadShape,
    PadShapeSubKind, PadStackMode, PcbFileFormatVersion, PcbFlags, PcbObjectId,
    PlaneConnectionStyle, RegionKind, RuleKind, TCacheState, TextKind, V6Layer, V7Layer,
};
pub use sch::{
    ConnectorKind, ConnectorState, HarnessBrush, HarnessCavityPartType,
    HarnessConnectionPointStyle, HarnessLengthUnit, HarnessShieldStyle, HarnessSpliceStyle,
    HarnessWireLengthType, IeeeSymbol, LeftRightSide, LineShape, LineStyle, NoErcSymbol,
    ParameterReadOnlyState, ParameterSetStyle, ParameterType, PenWidth, PinElectricalType,
    SchDisplaySettings, SchDisplayStyle, SchRecordType, SheetBorderStyle, SheetOrientation,
    SheetReferenceZoneStyle, SheetStyle, SheetSymbolType, StdLogicState, TextHorzAnchor,
    TextJustification, TextVertAnchor, VisibleGridStyle,
};
pub use unique_id::{UniqueId, UniqueIdError};
