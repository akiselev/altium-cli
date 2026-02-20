//! Schematic enum types for the v2 API.
//!
//! Each enum maps 1:1 to its C# counterpart with discriminant values
//! matching the serialized integer representation. Uses `#[altium_enum]`
//! for automatic `AltiumEnum` + `ParamCodec` derivation.

use altium_format_derive::altium_enum;

// ---------------------------------------------------------------------------
// PinElectricalType
// ---------------------------------------------------------------------------

/// Pin electrical type -- `TPinElectrical` from C#.
#[altium_enum]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum PinElectricalType {
    #[default]
    Input = 0,
    IO = 1,
    Output = 2,
    OpenCollector = 3,
    Passive = 4,
    HiZ = 5,
    OpenEmitter = 6,
    Power = 7,
}

// ---------------------------------------------------------------------------
// RotationBy90
// ---------------------------------------------------------------------------

/// Rotation by 90-degree increments -- `TRotationBy90` from C#.
#[altium_enum]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum RotationBy90 {
    #[default]
    Rotate0 = 0,
    Rotate90 = 1,
    Rotate180 = 2,
    Rotate270 = 3,
}

// ---------------------------------------------------------------------------
// LineStyle
// ---------------------------------------------------------------------------

/// Line style -- `TLineStyle` from C#.
#[altium_enum]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum LineStyle {
    #[default]
    Solid = 0,
    Dashed = 1,
    Dotted = 2,
    DashDotted = 3,
}

// ---------------------------------------------------------------------------
// IeeeSymbol
// ---------------------------------------------------------------------------

/// IEEE symbol type -- `TIeeeSymbol` from C# (used for pin symbols).
/// Sparse values require `#[altium(value = N)]`.
#[altium_enum]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum IeeeSymbol {
    #[default]
    #[altium(value = 0)]
    None,
    #[altium(value = 1)]
    Dot,
    #[altium(value = 2)]
    RightLeftSignalFlow,
    #[altium(value = 3)]
    Clock,
    #[altium(value = 4)]
    ActiveLowInput,
    #[altium(value = 5)]
    AnalogSignalIn,
    #[altium(value = 6)]
    NotLogicConnection,
    #[altium(value = 8)]
    PostponedOutput,
    #[altium(value = 9)]
    OpenCollector,
    #[altium(value = 10)]
    HiZ,
    #[altium(value = 11)]
    HighCurrent,
    #[altium(value = 12)]
    Pulse,
    #[altium(value = 13)]
    Schmitt,
    #[altium(value = 17)]
    OpenCollectorPullup,
    #[altium(value = 22)]
    OpenEmitter,
    #[altium(value = 23)]
    OpenEmitterPullup,
    #[altium(value = 24)]
    ShiftLeft,
    #[altium(value = 25)]
    OpenOutput,
    #[altium(value = 33)]
    LeftRightSignalFlow,
    #[altium(value = 34)]
    BiDirectionalSignalFlow,
}

// ---------------------------------------------------------------------------
// PortArrowStyle
// ---------------------------------------------------------------------------

/// Port arrow style -- `TPortArrowStyle` from C#.
#[altium_enum]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum PortArrowStyle {
    #[default]
    None = 0,
    Left = 1,
    Right = 2,
    LeftRight = 3,
    NoneVertical = 4,
    Top = 5,
    Bottom = 6,
    TopBottom = 7,
}

// ---------------------------------------------------------------------------
// PortIO
// ---------------------------------------------------------------------------

/// Port I/O direction -- `TPortIO` from C#.
#[altium_enum]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum PortIO {
    #[default]
    Unspecified = 0,
    Output = 1,
    Input = 2,
    Bidirectional = 3,
}

// ---------------------------------------------------------------------------
// PowerObjectStyle
// ---------------------------------------------------------------------------

/// Power object style -- `TPowerObjectStyle` from C#.
#[altium_enum]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum PowerObjectStyle {
    #[default]
    Circle = 0,
    Arrow = 1,
    Bar = 2,
    Wave = 3,
    GndPower = 4,
    GndSignal = 5,
    GndEarth = 6,
    GOSTArrow = 7,
    GOSTGndPower = 8,
    GOSTGndEarth = 9,
    GOSTBar = 10,
}

// ---------------------------------------------------------------------------
// TextJustification
// ---------------------------------------------------------------------------

/// Text justification -- `TTextJustification` from C#.
#[altium_enum]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextJustification {
    #[default]
    BottomLeft = 0,
    BottomCenter = 1,
    BottomRight = 2,
    CenterLeft = 3,
    Center = 4,
    CenterRight = 5,
    TopLeft = 6,
    TopCenter = 7,
    TopRight = 8,
}

// ---------------------------------------------------------------------------
// SheetStyle
// ---------------------------------------------------------------------------

/// Sheet style -- `TSheetStyle` from C#.
#[altium_enum]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum SheetStyle {
    #[default]
    A4 = 0,
    A3 = 1,
    A2 = 2,
    A1 = 3,
    A0 = 4,
    A = 5,
    B = 6,
    C = 7,
    D = 8,
    E = 9,
    Letter = 10,
    Legal = 11,
    Tabloid = 12,
    OrcadA = 13,
    OrcadB = 14,
    OrcadC = 15,
    OrcadD = 16,
    OrcadE = 17,
}

// ---------------------------------------------------------------------------
// PinItemMode
// ---------------------------------------------------------------------------

/// Pin item mode -- `TPinItemMode` from C#.
#[altium_enum]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum PinItemMode {
    #[default]
    Default = 0,
    Custom = 1,
}

// ---------------------------------------------------------------------------
// PinTextRotationAnchor
// ---------------------------------------------------------------------------

/// Pin text rotation anchor -- used for name/designator custom positioning.
#[altium_enum]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum PinTextRotationAnchor {
    #[default]
    Pin = 0,
    Component = 1,
}

// ---------------------------------------------------------------------------
// ComponentKind
// ---------------------------------------------------------------------------

/// Component kind -- `TComponentKind` from C#.
/// Sparse values (0, 1, 2, 5, 6) require `#[altium(value = N)]`.
#[altium_enum]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum ComponentKind {
    #[default]
    #[altium(value = 0)]
    Standard,
    #[altium(value = 1)]
    Mechanical,
    #[altium(value = 2)]
    Graphical,
    #[altium(value = 5)]
    StandardNoBOM,
    #[altium(value = 6)]
    Jumper,
}

// ---------------------------------------------------------------------------
// Size
// ---------------------------------------------------------------------------

/// Size (line width) -- `TSize` from C#.
#[altium_enum]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum Size {
    #[default]
    Smallest = 0,
    Small = 1,
    Medium = 2,
    Large = 3,
}

// ---------------------------------------------------------------------------
// NoERCSymbol
// ---------------------------------------------------------------------------

/// NoERC symbol type -- `TNoERCSymbol` from C#.
#[altium_enum]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum NoERCSymbol {
    #[default]
    CrossThin = 0,
    CrossThick = 1,
    CrossSmall = 2,
    CheckBox = 3,
    Triangle = 4,
}

// ---------------------------------------------------------------------------
// ParameterType
// ---------------------------------------------------------------------------

/// Parameter type -- `TParameterType` from C#.
#[altium_enum]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum ParameterType {
    #[default]
    String = 0,
    Integer = 1,
}

// ---------------------------------------------------------------------------
// ParameterReadOnlyState
// ---------------------------------------------------------------------------

/// Parameter read-only state -- `TParameter_ReadOnlyState` from C#.
#[altium_enum]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum ParameterReadOnlyState {
    #[default]
    ReadWrite = 0,
    ReadOnly = 1,
}

// ---------------------------------------------------------------------------
// StdLogicState
// ---------------------------------------------------------------------------

/// Formal type for pins -- `TStdLogicState` from C#.
#[altium_enum]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum StdLogicState {
    #[default]
    DontCare = 0,
    Low = 1,
    High = 2,
    HighZ = 3,
    Weak = 4,
    WeakLow = 5,
    WeakHigh = 6,
    Unknown = 7,
    Uninitialized = 8,
}

// ---------------------------------------------------------------------------
// HorizontalAlign
// ---------------------------------------------------------------------------

/// Horizontal alignment.
#[altium_enum]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum HorizontalAlign {
    #[default]
    Left = 0,
    Center = 1,
    Right = 2,
}

// ---------------------------------------------------------------------------
// LineShape
// ---------------------------------------------------------------------------

/// Line shape for polylines.
#[altium_enum]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum LineShape {
    #[default]
    None = 0,
    Arrow = 1,
    SolidArrow = 2,
    Tail = 3,
    SolidTail = 4,
    Circle = 5,
    Square = 6,
}

// ---------------------------------------------------------------------------
// LeftRightSide
// ---------------------------------------------------------------------------

/// Left/right side indicator.
#[altium_enum]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum LeftRightSide {
    #[default]
    Left = 0,
    Right = 1,
}

// ---------------------------------------------------------------------------
// ParameterSetStyle
// ---------------------------------------------------------------------------

/// Parameter set style -- `TParameterSetStyle` from C#.
#[altium_enum]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum ParameterSetStyle {
    #[default]
    Name = 0,
    Flat = 1,
}

// ---------------------------------------------------------------------------
// TextHorzAnchor
// ---------------------------------------------------------------------------

/// Text horizontal anchor -- `TTextHorzAnchor` from C#.
#[altium_enum]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextHorzAnchor {
    #[default]
    None = 0,
    Left = 1,
    Center = 2,
    Right = 3,
}

// ---------------------------------------------------------------------------
// TextVertAnchor
// ---------------------------------------------------------------------------

/// Text vertical anchor -- `TTextVertAnchor` from C#.
#[altium_enum]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextVertAnchor {
    #[default]
    None = 0,
    Top = 1,
    Center = 2,
    Bottom = 3,
}

// ---------------------------------------------------------------------------
// PinConglomerateFlags (bitflags)
// ---------------------------------------------------------------------------

bitflags::bitflags! {
    /// Pin conglomerate flags -- packed bitfield from the PinConglomerate parameter.
    ///
    /// The lower 2 bits encode rotation (handled separately as RotationBy90),
    /// so the flags start at bit 2.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct PinConglomerateFlags: u32 {
        /// Pin edge inner dot symbol.
        const PIN_HIDDEN            = 0x04;
        /// Show pin name.
        const SHOW_NAME             = 0x08;
        /// Show pin designator.
        const SHOW_DESIGNATOR       = 0x10;
        /// Pin is not accessible (inverted from IsNotAccessible).
        const NOT_ACCESSIBLE        = 0x20;
        /// Pin is graphically locked.
        const GRAPHICALLY_LOCKED    = 0x40;
        /// Owner index additional list.
        const OWNER_INDEX_ADDITIONAL_LIST = 0x80;
    }
}

impl crate::traits::ParamCodec for PinConglomerateFlags {
    fn read(params: &crate::parameters::ParameterCollection, key: &str) -> Option<Self> {
        params
            .get(key)
            .map(|v| Self::from_bits_truncate(v.as_int_or(0) as u32))
    }

    fn write(&self, params: &mut crate::parameters::ParameterCollection, key: &str) {
        params.add_int(key, self.bits() as i32);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parameters::ParameterCollection;
    use crate::traits::{AltiumEnum, ParamCodec};

    #[test]
    fn pin_electrical_type_roundtrip() {
        assert_eq!(PinElectricalType::from_int(0), PinElectricalType::Input);
        assert_eq!(PinElectricalType::from_int(1), PinElectricalType::IO);
        assert_eq!(PinElectricalType::from_int(7), PinElectricalType::Power);
        assert_eq!(PinElectricalType::Input.to_int(), 0);
        assert_eq!(PinElectricalType::Power.to_int(), 7);
        // Unknown value falls back to first variant
        assert_eq!(PinElectricalType::from_int(99), PinElectricalType::Input);
    }

    #[test]
    fn rotation_by90_roundtrip() {
        assert_eq!(RotationBy90::from_int(0), RotationBy90::Rotate0);
        assert_eq!(RotationBy90::from_int(3), RotationBy90::Rotate270);
        assert_eq!(RotationBy90::Rotate90.to_int(), 1);
    }

    #[test]
    fn ieee_symbol_sparse_values() {
        assert_eq!(IeeeSymbol::from_int(0), IeeeSymbol::None);
        assert_eq!(IeeeSymbol::from_int(8), IeeeSymbol::PostponedOutput);
        assert_eq!(IeeeSymbol::from_int(17), IeeeSymbol::OpenCollectorPullup);
        assert_eq!(IeeeSymbol::from_int(33), IeeeSymbol::LeftRightSignalFlow);
        assert_eq!(
            IeeeSymbol::from_int(34),
            IeeeSymbol::BiDirectionalSignalFlow
        );
        assert_eq!(IeeeSymbol::PostponedOutput.to_int(), 8);
        assert_eq!(IeeeSymbol::OpenCollectorPullup.to_int(), 17);
        // Unknown sparse value falls back to first variant
        assert_eq!(IeeeSymbol::from_int(7), IeeeSymbol::None);
    }

    #[test]
    fn component_kind_sparse_values() {
        assert_eq!(ComponentKind::from_int(0), ComponentKind::Standard);
        assert_eq!(ComponentKind::from_int(5), ComponentKind::StandardNoBOM);
        assert_eq!(ComponentKind::from_int(6), ComponentKind::Jumper);
        assert_eq!(ComponentKind::StandardNoBOM.to_int(), 5);
        assert_eq!(ComponentKind::Jumper.to_int(), 6);
        // Unknown sparse value falls back to first variant
        assert_eq!(ComponentKind::from_int(3), ComponentKind::Standard);
    }

    #[test]
    fn enum_param_codec_roundtrip() {
        let mut params = ParameterCollection::new();
        PinElectricalType::IO.write(&mut params, "Electrical");
        let read_back = PinElectricalType::read(&params, "Electrical");
        assert_eq!(read_back, Some(PinElectricalType::IO));
    }

    #[test]
    fn line_style_all_values() {
        assert_eq!(LineStyle::from_int(0), LineStyle::Solid);
        assert_eq!(LineStyle::from_int(1), LineStyle::Dashed);
        assert_eq!(LineStyle::from_int(2), LineStyle::Dotted);
        assert_eq!(LineStyle::from_int(3), LineStyle::DashDotted);
    }

    #[test]
    fn text_justification_all_values() {
        assert_eq!(
            TextJustification::from_int(0),
            TextJustification::BottomLeft
        );
        assert_eq!(TextJustification::from_int(4), TextJustification::Center);
        assert_eq!(TextJustification::from_int(8), TextJustification::TopRight);
    }

    #[test]
    fn sheet_style_all_values() {
        assert_eq!(SheetStyle::from_int(0), SheetStyle::A4);
        assert_eq!(SheetStyle::from_int(17), SheetStyle::OrcadE);
    }

    #[test]
    fn size_all_values() {
        assert_eq!(Size::from_int(0), Size::Smallest);
        assert_eq!(Size::from_int(3), Size::Large);
    }

    #[test]
    fn pin_conglomerate_flags_roundtrip() {
        let flags = PinConglomerateFlags::PIN_HIDDEN | PinConglomerateFlags::SHOW_NAME;
        let mut params = ParameterCollection::new();
        flags.write(&mut params, "PinConglomerate");
        let read_back = PinConglomerateFlags::read(&params, "PinConglomerate");
        assert_eq!(read_back, Some(flags));
    }

    #[test]
    fn pin_conglomerate_flags_truncation() {
        // Bits 0-1 are rotation, should be truncated
        let flags = PinConglomerateFlags::from_bits_truncate(0xFF);
        assert!(flags.contains(PinConglomerateFlags::PIN_HIDDEN));
        assert!(flags.contains(PinConglomerateFlags::SHOW_NAME));
        assert!(flags.contains(PinConglomerateFlags::SHOW_DESIGNATOR));
        assert!(flags.contains(PinConglomerateFlags::NOT_ACCESSIBLE));
        assert!(flags.contains(PinConglomerateFlags::GRAPHICALLY_LOCKED));
        assert!(flags.contains(PinConglomerateFlags::OWNER_INDEX_ADDITIONAL_LIST));
    }

    #[test]
    fn power_object_style_all_values() {
        assert_eq!(PowerObjectStyle::from_int(0), PowerObjectStyle::Circle);
        assert_eq!(PowerObjectStyle::from_int(10), PowerObjectStyle::GOSTBar);
    }

    #[test]
    fn port_io_all_values() {
        assert_eq!(PortIO::from_int(0), PortIO::Unspecified);
        assert_eq!(PortIO::from_int(3), PortIO::Bidirectional);
    }

    #[test]
    fn std_logic_state_all_values() {
        assert_eq!(StdLogicState::from_int(0), StdLogicState::DontCare);
        assert_eq!(StdLogicState::from_int(8), StdLogicState::Uninitialized);
    }

    #[test]
    fn text_anchors_all_values() {
        assert_eq!(TextHorzAnchor::from_int(0), TextHorzAnchor::None);
        assert_eq!(TextHorzAnchor::from_int(3), TextHorzAnchor::Right);
        assert_eq!(TextVertAnchor::from_int(0), TextVertAnchor::None);
        assert_eq!(TextVertAnchor::from_int(3), TextVertAnchor::Bottom);
    }
}
