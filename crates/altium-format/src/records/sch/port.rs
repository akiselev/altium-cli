//! SchPort - Schematic port object (Record 18).
//!
//! A port is a labelled connection point used to connect nets across sheets.

use crate::error::Result;
use crate::traits::{FromParamValue, FromParams, ToParamValue, ToParams};
use crate::types::ParameterValue;
use crate::types::{Coord, CoordRect, ParameterCollection, UnknownFields};
use altium_format_derive::AltiumRecord;

use super::{SchGraphicalBase, SchPrimitive};

/// Port style determining the visual appearance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum PortStyle {
    /// No arrow indicators.
    #[default]
    None = 0,
    /// Extends leftward.
    Left = 1,
    /// Extends rightward.
    Right = 2,
    /// Extends downward.
    Top = 3,
    /// Extends upward (value 7 in format doc).
    Bottom = 4,
    /// Internal value 3 - extends rightward.
    ExtendRight = 5,
    /// Internal value 7 - extends upwards.
    ExtendUp = 6,
}

impl PortStyle {
    pub fn from_int(value: i32) -> Self {
        match value {
            0 => PortStyle::None,
            1 => PortStyle::Left,
            2 => PortStyle::Right,
            3 => PortStyle::ExtendRight,
            4 => PortStyle::Top,
            5 | 6 => PortStyle::Bottom,
            7 => PortStyle::ExtendUp,
            _ => PortStyle::None,
        }
    }

    pub fn to_int(self) -> i32 {
        match self {
            PortStyle::None => 0,
            PortStyle::Left => 1,
            PortStyle::Right => 2,
            PortStyle::ExtendRight => 3,
            PortStyle::Top => 4,
            PortStyle::Bottom => 5,
            PortStyle::ExtendUp => 7,
        }
    }
}

impl FromParamValue for PortStyle {
    fn from_param_value(value: &ParameterValue) -> Result<Self> {
        Ok(Self::from_int(value.as_int_or(0)))
    }
}

impl ToParamValue for PortStyle {
    fn to_param_value(&self) -> String {
        self.to_int().to_string()
    }
}

/// Port I/O type determining the shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum PortIoType {
    /// Unspecified (flat left edge with angled right).
    #[default]
    Unspecified = 0,
    /// Output port.
    Output = 1,
    /// Input port.
    Input = 2,
    /// Bidirectional port (angled points on both ends).
    Bidirectional = 3,
}

impl PortIoType {
    pub fn from_int(value: i32) -> Self {
        match value {
            0 => PortIoType::Unspecified,
            1 => PortIoType::Output,
            2 => PortIoType::Input,
            3 => PortIoType::Bidirectional,
            _ => PortIoType::Unspecified,
        }
    }

    pub fn to_int(self) -> i32 {
        self as i32
    }
}

impl FromParamValue for PortIoType {
    fn from_param_value(value: &ParameterValue) -> Result<Self> {
        Ok(Self::from_int(value.as_int_or(0)))
    }
}

impl ToParamValue for PortIoType {
    fn to_param_value(&self) -> String {
        self.to_int().to_string()
    }
}

/// Port text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum PortAlignment {
    /// Unknown/default alignment.
    #[default]
    Unknown = 0,
    /// Text right-aligned.
    Right = 1,
    /// Text left-aligned.
    Left = 2,
}

impl PortAlignment {
    pub fn from_int(value: i32) -> Self {
        match value {
            0 => PortAlignment::Unknown,
            1 => PortAlignment::Right,
            2 => PortAlignment::Left,
            _ => PortAlignment::Unknown,
        }
    }

    pub fn to_int(self) -> i32 {
        self as i32
    }
}

impl FromParamValue for PortAlignment {
    fn from_param_value(value: &ParameterValue) -> Result<Self> {
        Ok(Self::from_int(value.as_int_or(0)))
    }
}

impl ToParamValue for PortAlignment {
    fn to_param_value(&self) -> String {
        self.to_int().to_string()
    }
}

/// Schematic port primitive - labelled connection point.
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(record_id = 18, format = "params")]
pub struct SchPort {
    /// Graphical base (location, color).
    #[altium(flatten)]
    pub graphical: SchGraphicalBase,

    /// Port style determining visual appearance.
    #[altium(param = "STYLE", default)]
    pub style: PortStyle,

    /// Port I/O type determining shape.
    #[altium(param = "IOTYPE", default)]
    pub io_type: PortIoType,

    /// Text alignment.
    #[altium(param = "ALIGNMENT", default)]
    pub alignment: PortAlignment,

    /// Port width.
    #[altium(param = "WIDTH", default)]
    pub width: i32,

    /// Port height.
    #[altium(param = "HEIGHT", default)]
    pub height: i32,

    /// Port name/label text (backslash indicates overline).
    #[altium(param = "NAME", default)]
    pub name: String,

    /// Text color.
    #[altium(param = "TEXTCOLOR", default)]
    pub text_color: i32,

    /// Font ID (zero uses default).
    #[altium(param = "FONTID", default)]
    pub font_id: i32,

    /// Unique identifier.
    #[altium(param = "UNIQUEID", default)]
    pub unique_id: String,

    /// Harness type name (omitted if normal signal).
    #[altium(param = "HARNESSTYPE", default)]
    pub harness_type: String,

    /// Unknown parameters (preserved for non-destructive editing).
    #[altium(unknown)]
    pub unknown_params: UnknownFields,
}

impl SchPrimitive for SchPort {
    const RECORD_ID: i32 = 18;

    fn location(&self) -> Option<crate::types::CoordPoint> {
        Some(crate::types::CoordPoint::from_raw(
            self.graphical.location_x,
            self.graphical.location_y,
        ))
    }

    fn record_type_name(&self) -> &'static str {
        "Port"
    }

    fn get_property(&self, name: &str) -> Option<String> {
        match name {
            "NAME" => Some(self.name.clone()),
            _ => None,
        }
    }

    fn import_from_params(params: &ParameterCollection) -> Result<Self> {
        Self::from_params(params)
    }

    fn export_to_params(&self) -> ParameterCollection {
        self.to_params()
    }

    fn owner_index(&self) -> i32 {
        self.graphical.base.owner_index
    }

    fn calculate_bounds(&self) -> CoordRect {
        CoordRect::from_points(
            Coord::from_raw(self.graphical.location_x),
            Coord::from_raw(self.graphical.location_y),
            Coord::from_raw(self.graphical.location_x + self.width * 10000),
            Coord::from_raw(self.graphical.location_y + self.height * 10000),
        )
    }
}
