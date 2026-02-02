//! PCB Board record (parametric only).
//!
//! Board6/Data contains board settings: layer stack, sheet size, grid, origin.
//!
//! Key fields: Record=Board, FileName, Kind, Version, Date, Time,
//! OriginX, OriginY, grid settings, layer stack (LAYERSETSCOUNT, per-layer fields).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::coord::PcbCoord;

/// PCB Board record (parametric).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PcbBoard {
    pub properties: HashMap<String, String>,
}

impl PcbBoard {
    pub fn from_properties(props: HashMap<String, String>) -> Self {
        Self { properties: props }
    }

    pub fn origin_x(&self) -> Option<PcbCoord> {
        self.properties.get("ORIGINX").and_then(|s| s.parse::<i32>().ok()).map(PcbCoord::from_raw)
    }

    pub fn origin_y(&self) -> Option<PcbCoord> {
        self.properties.get("ORIGINY").and_then(|s| s.parse::<i32>().ok()).map(PcbCoord::from_raw)
    }

    pub fn display_unit(&self) -> Option<u8> {
        self.properties.get("DISPLAYUNIT").and_then(|s| s.parse().ok())
    }

    pub fn snap_grid_size(&self) -> Option<f64> {
        self.properties.get("SNAPGRIDSIZE").and_then(|s| s.parse().ok())
    }

    pub fn layer_sets_count(&self) -> Option<u32> {
        self.properties.get("LAYERSETSCOUNT").and_then(|s| s.parse().ok())
    }
}
