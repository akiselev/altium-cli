//! PCB Dimension record (ID=13, parametric only).
//!
//! Dimensions6/Data uses parametric format.
//! Key fields: KIND, LAYER, text format/position, reference/text points, UNITS.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::enums::TDimensionKind;

/// PCB Dimension record (parametric).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PcbDimension {
    pub properties: HashMap<String, String>,
}

impl PcbDimension {
    pub fn from_properties(props: HashMap<String, String>) -> Self {
        Self { properties: props }
    }

    pub fn kind(&self) -> Option<TDimensionKind> {
        self.properties.get("DIMENSIONKIND")
            .or_else(|| self.properties.get("KIND"))
            .and_then(|s| s.parse::<u8>().ok())
            .and_then(TDimensionKind::from_u8)
    }

    pub fn layer(&self) -> Option<u8> {
        self.properties.get("LAYER").and_then(|s| s.parse().ok())
    }

    pub fn text_x(&self) -> Option<i32> {
        self.properties.get("TEXTX").and_then(|s| s.parse().ok())
    }

    pub fn text_y(&self) -> Option<i32> {
        self.properties.get("TEXTY").and_then(|s| s.parse().ok())
    }
}
