//! Conversion traits between raw Altium parameter strings and typed Rust values.
//! `FromParamValue`: parse a string value for a named key into `T`.
//! `ToParamValue`: serialize `T` back to the Altium string representation.
//! `bool` uses Altium's T/F encoding, not Rust's true/false.
use altium_format_types::Coord;

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
        if *self { "T".to_owned() } else { "F".to_owned() }
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
            AltiumFormatError::InvalidParamValue { key: key.to_owned(), detail: e.to_string() }
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
        value.parse::<usize>().map_err(|e| AltiumFormatError::InvalidParamValue {
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
