//! Common types and enums for schematic records.

use bitflags::bitflags;

/// Line width for schematic primitives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum LineWidth {
    #[default]
    Smallest = 0,
    Small = 1,
    Medium = 2,
    Large = 3,
}

impl LineWidth {
    pub fn from_int(value: i32) -> Self {
        match value {
            0 => LineWidth::Smallest,
            1 => LineWidth::Small,
            2 => LineWidth::Medium,
            3 => LineWidth::Large,
            _ => LineWidth::Smallest,
        }
    }

    pub fn to_int(self) -> i32 {
        self as i32
    }
}

/// Line style for polylines and wires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum LineStyle {
    #[default]
    Solid = 0,
    Dashed = 1,
    Dotted = 2,
}

impl LineStyle {
    pub fn from_int(value: i32) -> Self {
        match value {
            0 => LineStyle::Solid,
            1 => LineStyle::Dashed,
            2 => LineStyle::Dotted,
            _ => LineStyle::Solid,
        }
    }

    pub fn to_int(self) -> i32 {
        self as i32
    }
}

bitflags! {
    /// Text orientation flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct TextOrientations: u8 {
        const NONE = 0;
        const ROTATED = 1;
        const FLIPPED = 2;
    }
}

impl TextOrientations {
    pub fn from_int(value: i32) -> Self {
        TextOrientations::from_bits_truncate(value as u8)
    }

    pub fn to_int(self) -> i32 {
        self.bits() as i32
    }
}

/// Text justification (3x3 grid: horizontal x vertical).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextJustification(i32);

impl TextJustification {
    pub const BOTTOM_LEFT: Self = TextJustification(0);
    pub const BOTTOM_CENTER: Self = TextJustification(1);
    pub const BOTTOM_RIGHT: Self = TextJustification(2);
    pub const MIDDLE_LEFT: Self = TextJustification(3);
    pub const MIDDLE_CENTER: Self = TextJustification(4);
    pub const MIDDLE_RIGHT: Self = TextJustification(5);
    pub const TOP_LEFT: Self = TextJustification(6);
    pub const TOP_CENTER: Self = TextJustification(7);
    pub const TOP_RIGHT: Self = TextJustification(8);

    pub fn from_int(value: i32) -> Self {
        TextJustification(value.clamp(0, 8))
    }

    pub fn to_int(self) -> i32 {
        self.0
    }

    /// Horizontal component: Left=0, Center=1, Right=2
    pub fn horizontal(self) -> i32 {
        self.0 % 3
    }

    /// Vertical component: Bottom=0, Middle=1, Top=2
    pub fn vertical(self) -> i32 {
        self.0 / 3
    }
}

/// Pin electrical type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum PinElectricalType {
    Input = 0,
    InputOutput = 1,
    Output = 2,
    OpenCollector = 3,
    #[default]
    Passive = 4,
    HiZ = 5,
    OpenEmitter = 6,
    Power = 7,
}

impl PinElectricalType {
    pub fn from_int(value: i32) -> Self {
        match value {
            0 => PinElectricalType::Input,
            1 => PinElectricalType::InputOutput,
            2 => PinElectricalType::Output,
            3 => PinElectricalType::OpenCollector,
            4 => PinElectricalType::Passive,
            5 => PinElectricalType::HiZ,
            6 => PinElectricalType::OpenEmitter,
            7 => PinElectricalType::Power,
            _ => PinElectricalType::Passive,
        }
    }

    pub fn to_int(self) -> i32 {
        self as i32
    }
}

/// Pin symbol type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum PinSymbol {
    #[default]
    None = 0,
    Dot = 1,
    RightLeftSignalFlow = 2,
    Clock = 3,
    ActiveLowInput = 4,
    AnalogSignalIn = 5,
    NotLogicConnection = 6,
    PostponedOutput = 8,
    OpenCollector = 9,
    HiZ = 10,
    HighCurrent = 11,
    Pulse = 12,
    Schmitt = 13,
    ActiveLowOutput = 17,
    OpenCollectorPullUp = 22,
    OpenEmitter = 23,
    OpenEmitterPullUp = 24,
    DigitalSignalIn = 25,
    ShiftLeft = 30,
    OpenOutput = 32,
    LeftRightSignalFlow = 33,
    BidirectionalSignalFlow = 34,
}

impl PinSymbol {
    pub fn from_int(value: i32) -> Self {
        match value {
            0 => PinSymbol::None,
            1 => PinSymbol::Dot,
            2 => PinSymbol::RightLeftSignalFlow,
            3 => PinSymbol::Clock,
            4 => PinSymbol::ActiveLowInput,
            5 => PinSymbol::AnalogSignalIn,
            6 => PinSymbol::NotLogicConnection,
            8 => PinSymbol::PostponedOutput,
            9 => PinSymbol::OpenCollector,
            10 => PinSymbol::HiZ,
            11 => PinSymbol::HighCurrent,
            12 => PinSymbol::Pulse,
            13 => PinSymbol::Schmitt,
            17 => PinSymbol::ActiveLowOutput,
            22 => PinSymbol::OpenCollectorPullUp,
            23 => PinSymbol::OpenEmitter,
            24 => PinSymbol::OpenEmitterPullUp,
            25 => PinSymbol::DigitalSignalIn,
            30 => PinSymbol::ShiftLeft,
            32 => PinSymbol::OpenOutput,
            33 => PinSymbol::LeftRightSignalFlow,
            34 => PinSymbol::BidirectionalSignalFlow,
            _ => PinSymbol::None,
        }
    }

    pub fn to_int(self) -> i32 {
        self as i32
    }
}

bitflags! {
    /// Pin conglomerate flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct PinConglomerateFlags: u8 {
        const NONE = 0x00;
        const ROTATED = 0x01;
        const FLIPPED = 0x02;
        const HIDE = 0x04;
        const DISPLAY_NAME_VISIBLE = 0x08;
        const DESIGNATOR_VISIBLE = 0x10;
        const UNKNOWN = 0x20;
        const GRAPHICALLY_LOCKED = 0x40;
    }
}

impl PinConglomerateFlags {
    pub fn from_int(value: i32) -> Self {
        PinConglomerateFlags::from_bits_truncate(value as u8)
    }

    pub fn to_int(self) -> i32 {
        self.bits() as i32
    }
}

/// Power object style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum PowerObjectStyle {
    #[default]
    Arrow = 0,
    Bar = 1,
    Wave = 2,
    Ground = 3,
    PowerGround = 4,
    SignalGround = 5,
    EarthGround = 6,
    Circle = 7,
}

impl PowerObjectStyle {
    pub fn from_int(value: i32) -> Self {
        match value {
            0 => PowerObjectStyle::Arrow,
            1 => PowerObjectStyle::Bar,
            2 => PowerObjectStyle::Wave,
            3 => PowerObjectStyle::Ground,
            4 => PowerObjectStyle::PowerGround,
            5 => PowerObjectStyle::SignalGround,
            6 => PowerObjectStyle::EarthGround,
            7 => PowerObjectStyle::Circle,
            _ => PowerObjectStyle::Arrow,
        }
    }

    pub fn to_int(self) -> i32 {
        self as i32
    }
}

/// Utility to convert DXP integer + fraction to Coord raw value.
///
/// Altium stores coordinates as integer mils plus a fractional part.
/// Raw coord = (integer * 10000) + fraction
pub fn dxp_frac_to_coord(integer: i32, frac: i32) -> i32 {
    integer * 10000 + frac
}

/// Utility to convert Coord raw value to DXP integer + fraction.
pub fn coord_to_dxp_frac(raw: i32) -> (i32, i32) {
    (raw / 10000, raw % 10000)
}

// ============================================================================
// Trait implementations for derive macro support
// ============================================================================

use crate::error::Result;
use crate::traits::{FromParamValue, ToParamValue};
use crate::types::ParameterValue;

impl FromParamValue for LineWidth {
    fn from_param_value(value: &ParameterValue) -> Result<Self> {
        Ok(Self::from_int(value.as_int_or(0)))
    }
}

impl ToParamValue for LineWidth {
    fn to_param_value(&self) -> String {
        self.to_int().to_string()
    }
}

impl FromParamValue for LineStyle {
    fn from_param_value(value: &ParameterValue) -> Result<Self> {
        Ok(Self::from_int(value.as_int_or(0)))
    }
}

impl ToParamValue for LineStyle {
    fn to_param_value(&self) -> String {
        self.to_int().to_string()
    }
}

impl FromParamValue for PinElectricalType {
    fn from_param_value(value: &ParameterValue) -> Result<Self> {
        Ok(Self::from_int(value.as_int_or(0)))
    }
}

impl ToParamValue for PinElectricalType {
    fn to_param_value(&self) -> String {
        self.to_int().to_string()
    }
}

impl FromParamValue for PinSymbol {
    fn from_param_value(value: &ParameterValue) -> Result<Self> {
        Ok(Self::from_int(value.as_int_or(0)))
    }
}

impl ToParamValue for PinSymbol {
    fn to_param_value(&self) -> String {
        self.to_int().to_string()
    }
}

impl FromParamValue for PinConglomerateFlags {
    fn from_param_value(value: &ParameterValue) -> Result<Self> {
        Ok(Self::from_int(value.as_int_or(0)))
    }
}

impl ToParamValue for PinConglomerateFlags {
    fn to_param_value(&self) -> String {
        self.to_int().to_string()
    }
}

impl FromParamValue for TextOrientations {
    fn from_param_value(value: &ParameterValue) -> Result<Self> {
        Ok(Self::from_int(value.as_int_or(0)))
    }
}

impl ToParamValue for TextOrientations {
    fn to_param_value(&self) -> String {
        self.to_int().to_string()
    }
}

impl FromParamValue for TextJustification {
    fn from_param_value(value: &ParameterValue) -> Result<Self> {
        Ok(Self::from_int(value.as_int_or(0)))
    }
}

impl ToParamValue for TextJustification {
    fn to_param_value(&self) -> String {
        self.to_int().to_string()
    }
}

impl FromParamValue for PowerObjectStyle {
    fn from_param_value(value: &ParameterValue) -> Result<Self> {
        Ok(Self::from_int(value.as_int_or(0)))
    }
}

impl ToParamValue for PowerObjectStyle {
    fn to_param_value(&self) -> String {
        self.to_int().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_justification() {
        let tj = TextJustification::MIDDLE_CENTER;
        assert_eq!(tj.horizontal(), 1);
        assert_eq!(tj.vertical(), 1);

        let tj = TextJustification::TOP_RIGHT;
        assert_eq!(tj.horizontal(), 2);
        assert_eq!(tj.vertical(), 2);
    }

    #[test]
    fn test_dxp_frac_conversion() {
        let raw = dxp_frac_to_coord(100, 5000);
        assert_eq!(raw, 1005000);

        let (integer, frac) = coord_to_dxp_frac(1005000);
        assert_eq!(integer, 100);
        assert_eq!(frac, 5000);
    }
}
