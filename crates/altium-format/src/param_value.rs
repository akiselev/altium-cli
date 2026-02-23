//! Conversion traits between raw Altium parameter strings and typed Rust values.
//! `FromParamValue`: parse a string value for a named key into `T`.
//! `ToParamValue`: serialize `T` back to the Altium string representation.
//! `bool` uses Altium's T/F encoding, not Rust's true/false.
use altium_format_types::{
    Color, ComponentKind, Coord, IeeeSymbol, LeftRightSide, LineShape, LineStyle,
    ParameterReadOnlyState, ParameterType, PenWidth, RotationBy90, SheetSymbolType, TextHorzAnchor,
    TextJustification, TextVertAnchor, UniqueId,
    sch::{PortArrowStyle, PortIoType, PowerObjectStyle},
};

use crate::{AltiumFormatError, Result};

pub(crate) trait FromParamValue: Sized {
    fn from_param_value(key: &str, value: &str) -> Result<Self>;
}

pub(crate) trait ToParamValue {
    fn to_param_value(&self) -> String;
}

impl FromParamValue for String {
    fn from_param_value(_key: &str, value: &str) -> Result<Self> {
        Ok(value.to_owned())
    }
}

impl ToParamValue for String {
    fn to_param_value(&self) -> String {
        self.clone()
    }
}

impl FromParamValue for bool {
    fn from_param_value(key: &str, value: &str) -> Result<Self> {
        match value {
            "T" | "TRUE" => Ok(true),
            "F" | "FALSE" => Ok(false),
            other => Err(AltiumFormatError::InvalidParamValue {
                key: key.to_owned(),
                detail: format!("expected T/F/TRUE/FALSE, got {other:?}"),
            }),
        }
    }
}

impl ToParamValue for bool {
    fn to_param_value(&self) -> String {
        if *self {
            "T".to_owned()
        } else {
            "F".to_owned()
        }
    }
}

macro_rules! impl_int_param_value {
    ($($t:ty),+) => {
        $(
            impl FromParamValue for $t {
                fn from_param_value(key: &str, value: &str) -> Result<Self> {
                    value.parse::<$t>().map_err(|e| AltiumFormatError::InvalidParamValue {
                        key: key.to_owned(),
                        detail: e.to_string(),
                    })
                }
            }

            impl ToParamValue for $t {
                fn to_param_value(&self) -> String {
                    self.to_string()
                }
            }
        )+
    };
}

impl_int_param_value!(i8, u8, i16, u16, i32, u32, f64);

impl FromParamValue for Coord {
    fn from_param_value(key: &str, value: &str) -> Result<Self> {
        let raw: i32 = value.parse().map_err(|e: std::num::ParseIntError| {
            AltiumFormatError::InvalidParamValue {
                key: key.to_owned(),
                detail: e.to_string(),
            }
        })?;
        Ok(Coord::from_internal(raw))
    }
}

impl ToParamValue for Coord {
    fn to_param_value(&self) -> String {
        self.to_internal().to_string()
    }
}

// usize is excluded from impl_int_param_value! because its width is platform-dependent
// (32-bit or 64-bit depending on target); used for Weight and count fields.
impl FromParamValue for usize {
    fn from_param_value(key: &str, value: &str) -> Result<Self> {
        value
            .parse::<usize>()
            .map_err(|e| AltiumFormatError::InvalidParamValue {
                key: key.to_owned(),
                detail: e.to_string(),
            })
    }
}

impl ToParamValue for usize {
    fn to_param_value(&self) -> String {
        self.to_string()
    }
}

// Color is stored as a decimal Win32 COLORREF integer (0x00BBGGRR) in parameter strings.
impl FromParamValue for Color {
    fn from_param_value(key: &str, value: &str) -> Result<Self> {
        let raw: i32 = value.parse().map_err(|e: std::num::ParseIntError| {
            AltiumFormatError::InvalidParamValue {
                key: key.to_owned(),
                detail: e.to_string(),
            }
        })?;
        Ok(Color::new(raw))
    }
}

impl ToParamValue for Color {
    fn to_param_value(&self) -> String {
        self.raw().to_string()
    }
}

// Enum types that are stored as decimal integer discriminants in parameter strings.
// Each enum must implement TryFrom<u8> with Error = InvalidEnumValue.
macro_rules! impl_enum_param_value {
    ($($t:ty),+ $(,)?) => {
        $(
            impl FromParamValue for $t {
                fn from_param_value(key: &str, value: &str) -> Result<Self> {
                    let raw = u8::from_param_value(key, value)?;
                    Self::try_from(raw).map_err(|e: altium_format_types::InvalidEnumValue| {
                        AltiumFormatError::InvalidParamValue {
                            key: key.to_owned(),
                            detail: e.to_string(),
                        }
                    })
                }
            }

            impl ToParamValue for $t {
                fn to_param_value(&self) -> String {
                    (*self as u8).to_string()
                }
            }
        )+
    };
}

impl_enum_param_value!(
    PenWidth,
    LineStyle,
    LineShape,
    TextJustification,
    RotationBy90,
    ComponentKind,
    IeeeSymbol,
    ParameterReadOnlyState,
    ParameterType,
    TextHorzAnchor,
    TextVertAnchor,
    PowerObjectStyle,
    PortArrowStyle,
    PortIoType,
    LeftRightSide,
);

// Angle value that serializes with exactly 3 decimal places (matching Altium's N3 format).
// C#: StrUtils.DoubleToString(value, "N3") always produces "180.000", "45.000", etc.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SchAngle(pub f64);

impl ToParamValue for SchAngle {
    fn to_param_value(&self) -> String {
        format!("{:.3}", self.0)
    }
}

impl FromParamValue for SchAngle {
    fn from_param_value(key: &str, value: &str) -> Result<Self> {
        let v: f64 = value.parse().map_err(|e: std::num::ParseFloatError| {
            AltiumFormatError::InvalidParamValue {
                key: key.to_owned(),
                detail: format!("invalid angle: {e}"),
            }
        })?;
        Ok(SchAngle(v))
    }
}

impl Default for SchAngle {
    fn default() -> Self {
        SchAngle(0.0)
    }
}

// UniqueId is stored as an 8-char uppercase alpha string in parameter values.
impl FromParamValue for UniqueId {
    fn from_param_value(key: &str, value: &str) -> Result<Self> {
        value
            .parse::<UniqueId>()
            .map_err(|e| AltiumFormatError::InvalidParamValue {
                key: key.to_owned(),
                detail: e.to_string(),
            })
    }
}

impl ToParamValue for UniqueId {
    fn to_param_value(&self) -> String {
        self.as_str().to_owned()
    }
}

// SheetSymbolType is serialized as display strings in SchDoc text records.
// C# mapping (SchDataUtils.{StringTo,ToString}SheetSymbolType):
// Normal | Device Sheet | Design Item
impl FromParamValue for SheetSymbolType {
    fn from_param_value(key: &str, value: &str) -> Result<Self> {
        if value.eq_ignore_ascii_case("Normal") {
            return Ok(SheetSymbolType::Normal);
        }
        if value.eq_ignore_ascii_case("Device Sheet") {
            return Ok(SheetSymbolType::DeviceSheet);
        }
        if value.eq_ignore_ascii_case("Design Item") {
            return Ok(SheetSymbolType::DesignItem);
        }

        // Accept numeric fallback for robustness in mixed-version files.
        if let Ok(raw) = value.parse::<u8>() {
            return SheetSymbolType::try_from(raw).map_err(|e| {
                AltiumFormatError::InvalidParamValue {
                    key: key.to_owned(),
                    detail: e.to_string(),
                }
            });
        }

        Err(AltiumFormatError::InvalidParamValue {
            key: key.to_owned(),
            detail: format!("expected one of Normal/Device Sheet/Design Item or u8, got {value:?}"),
        })
    }
}

impl ToParamValue for SheetSymbolType {
    fn to_param_value(&self) -> String {
        match self {
            SheetSymbolType::Normal => "Normal".to_owned(),
            SheetSymbolType::DeviceSheet => "Device Sheet".to_owned(),
            SheetSymbolType::DesignItem => "Design Item".to_owned(),
            _ => (*self as u8).to_string(),
        }
    }
}
