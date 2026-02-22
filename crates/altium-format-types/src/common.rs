use std::fmt;

use crate::InvalidEnumValue;

/// Component classification, shared between schematic and PCB.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum ComponentKind {
    #[default]
    Standard = 0,
    Mechanical = 1,
    Graphical = 2,
    NetTieBom = 3,
    NetTieNoBom = 4,
    StandardNoBom = 5,
    Jumper = 6,
}

impl TryFrom<u8> for ComponentKind {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Standard),
            1 => Ok(Self::Mechanical),
            2 => Ok(Self::Graphical),
            3 => Ok(Self::NetTieBom),
            4 => Ok(Self::NetTieNoBom),
            5 => Ok(Self::StandardNoBom),
            6 => Ok(Self::Jumper),
            _ => Err(InvalidEnumValue { type_name: "ComponentKind", value: v as i64 }),
        }
    }
}

impl fmt::Display for ComponentKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Standard => write!(f, "Standard"),
            Self::Mechanical => write!(f, "Mechanical"),
            Self::Graphical => write!(f, "Graphical"),
            Self::NetTieBom => write!(f, "NetTieBom"),
            Self::NetTieNoBom => write!(f, "NetTieNoBom"),
            Self::StandardNoBom => write!(f, "StandardNoBom"),
            Self::Jumper => write!(f, "Jumper"),
        }
    }
}

/// Rotation in 90-degree increments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum RotationBy90 {
    #[default]
    Rotate0 = 0,
    Rotate90 = 1,
    Rotate180 = 2,
    Rotate270 = 3,
}

impl TryFrom<u8> for RotationBy90 {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Rotate0),
            1 => Ok(Self::Rotate90),
            2 => Ok(Self::Rotate180),
            3 => Ok(Self::Rotate270),
            _ => Err(InvalidEnumValue { type_name: "RotationBy90", value: v as i64 }),
        }
    }
}

impl fmt::Display for RotationBy90 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rotate0 => write!(f, "0°"),
            Self::Rotate90 => write!(f, "90°"),
            Self::Rotate180 => write!(f, "180°"),
            Self::Rotate270 => write!(f, "270°"),
        }
    }
}

/// Text auto-position (0-9), shared between schematic and PCB.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum TextAutoPosition {
    #[default]
    Manual = 0,
    TopLeft = 1,
    CenterLeft = 2,
    BottomLeft = 3,
    TopCenter = 4,
    CenterCenter = 5,
    BottomCenter = 6,
    TopRight = 7,
    CenterRight = 8,
    BottomRight = 9,
}

impl TryFrom<u8> for TextAutoPosition {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Manual),
            1 => Ok(Self::TopLeft),
            2 => Ok(Self::CenterLeft),
            3 => Ok(Self::BottomLeft),
            4 => Ok(Self::TopCenter),
            5 => Ok(Self::CenterCenter),
            6 => Ok(Self::BottomCenter),
            7 => Ok(Self::TopRight),
            8 => Ok(Self::CenterRight),
            9 => Ok(Self::BottomRight),
            _ => Err(InvalidEnumValue { type_name: "TextAutoPosition", value: v as i64 }),
        }
    }
}

impl fmt::Display for TextAutoPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Manual => write!(f, "Manual"),
            Self::TopLeft => write!(f, "TopLeft"),
            Self::CenterLeft => write!(f, "CenterLeft"),
            Self::BottomLeft => write!(f, "BottomLeft"),
            Self::TopCenter => write!(f, "TopCenter"),
            Self::CenterCenter => write!(f, "CenterCenter"),
            Self::BottomCenter => write!(f, "BottomCenter"),
            Self::TopRight => write!(f, "TopRight"),
            Self::CenterRight => write!(f, "CenterRight"),
            Self::BottomRight => write!(f, "BottomRight"),
        }
    }
}

/// Measurement unit preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum Unit {
    Metric = 0,
    #[default]
    Imperial = 1,
}

impl TryFrom<u8> for Unit {
    type Error = InvalidEnumValue;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Metric),
            1 => Ok(Self::Imperial),
            _ => Err(InvalidEnumValue { type_name: "Unit", value: v as i64 }),
        }
    }
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Metric => write!(f, "Metric"),
            Self::Imperial => write!(f, "Imperial"),
        }
    }
}
