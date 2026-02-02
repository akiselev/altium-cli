//! PCB Component record (ID=9, parametric only).
//!
//! Components6/Data uses pure parametric format (`u32 len + |KEY=VALUE|` ASCII text).
//!
//! Key fields from Ghidra (FUN_015f86b0):
//! PATTERN, SOURCEDESIGNATOR, X/Y location, ROTATION, LAYER,
//! NAMEON, COMMENTON, GROUPNUM, HEIGHT, etc.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::coord::PcbCoord;

/// PCB Component record (parametric).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PcbComponent {
    /// Raw parametric properties.
    pub properties: HashMap<String, String>,
}

impl PcbComponent {
    pub fn from_properties(props: HashMap<String, String>) -> Self {
        Self { properties: props }
    }

    pub fn pattern(&self) -> Option<&str> {
        self.properties.get("PATTERN").map(|s| s.as_str())
    }

    pub fn source_designator(&self) -> Option<&str> {
        self.properties.get("SOURCEDESIGNATOR").map(|s| s.as_str())
    }

    pub fn location_x(&self) -> Option<PcbCoord> {
        self.properties.get("X").and_then(|s| s.parse::<i32>().ok()).map(PcbCoord::from_raw)
            .or_else(|| self.properties.get("LOCATION.X").and_then(|s| s.parse::<i32>().ok()).map(PcbCoord::from_raw))
    }

    pub fn location_y(&self) -> Option<PcbCoord> {
        self.properties.get("Y").and_then(|s| s.parse::<i32>().ok()).map(PcbCoord::from_raw)
            .or_else(|| self.properties.get("LOCATION.Y").and_then(|s| s.parse::<i32>().ok()).map(PcbCoord::from_raw))
    }

    pub fn rotation(&self) -> Option<f64> {
        self.properties.get("ROTATION").and_then(|s| s.parse().ok())
    }

    pub fn layer(&self) -> Option<u8> {
        self.properties.get("LAYER").and_then(|s| s.parse().ok())
    }

    pub fn height(&self) -> Option<PcbCoord> {
        self.properties.get("HEIGHT").and_then(|s| s.parse::<i32>().ok()).map(PcbCoord::from_raw)
    }
}
