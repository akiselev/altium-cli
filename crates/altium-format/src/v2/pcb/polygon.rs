//! PCB Polygon record (ID=10, parametric only).
//!
//! Polygons6/Data uses parametric format with inline vertex data.
//!
//! Key fields: POLYGONTYPE, GRIDSIZE, TRACKWIDTH, HATCHSTYLE,
//! KIND0, VX0, VY0, CX0, CY0, SA0, EA0, R0, etc.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::coord::PcbCoord;

/// A polygon vertex (from inline parametric data).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PolygonVertex {
    /// Vertex kind (0=line, 1=arc).
    pub kind: u8,
    pub x: PcbCoord,
    pub y: PcbCoord,
    /// Arc center X (only if kind=1).
    pub cx: Option<PcbCoord>,
    /// Arc center Y (only if kind=1).
    pub cy: Option<PcbCoord>,
    /// Arc start angle (only if kind=1).
    pub sa: Option<f64>,
    /// Arc end angle (only if kind=1).
    pub ea: Option<f64>,
    /// Arc radius (only if kind=1).
    pub radius: Option<PcbCoord>,
}

/// PCB Polygon record (parametric).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PcbPolygon {
    pub properties: HashMap<String, String>,
}

impl PcbPolygon {
    pub fn from_properties(props: HashMap<String, String>) -> Self {
        Self { properties: props }
    }

    pub fn polygon_type(&self) -> Option<&str> {
        self.properties.get("POLYGONTYPE").map(|s| s.as_str())
    }

    pub fn grid_size(&self) -> Option<PcbCoord> {
        self.properties.get("GRIDSIZE").and_then(|s| s.parse::<i32>().ok()).map(PcbCoord::from_raw)
    }

    pub fn track_width(&self) -> Option<PcbCoord> {
        self.properties.get("TRACKWIDTH").and_then(|s| s.parse::<i32>().ok()).map(PcbCoord::from_raw)
    }

    pub fn hatch_style(&self) -> Option<u8> {
        self.properties.get("HATCHSTYLE").and_then(|s| s.parse().ok())
    }

    pub fn layer(&self) -> Option<&str> {
        self.properties.get("LAYER").map(|s| s.as_str())
    }

    pub fn net(&self) -> Option<&str> {
        self.properties.get("NET").map(|s| s.as_str())
    }

    /// Extract inline vertices from properties (VX0, VY0, KIND0, ...).
    pub fn vertices(&self) -> Vec<PolygonVertex> {
        let mut verts = Vec::new();
        let mut i = 0;
        loop {
            let vx_key = format!("VX{}", i);
            let vy_key = format!("VY{}", i);
            let (vx, vy) = match (self.properties.get(&vx_key), self.properties.get(&vy_key)) {
                (Some(vx), Some(vy)) => (vx, vy),
                _ => break,
            };
            let x = vx.parse::<i32>().unwrap_or(0);
            let y = vy.parse::<i32>().unwrap_or(0);
            let kind_key = format!("KIND{}", i);
            let kind = self.properties.get(&kind_key)
                .and_then(|s| s.parse::<u8>().ok())
                .unwrap_or(0);

            let (cx, cy, sa, ea, radius) = if kind == 1 {
                let cx = self.properties.get(&format!("CX{}", i)).and_then(|s| s.parse::<i32>().ok()).map(PcbCoord::from_raw);
                let cy = self.properties.get(&format!("CY{}", i)).and_then(|s| s.parse::<i32>().ok()).map(PcbCoord::from_raw);
                let sa = self.properties.get(&format!("SA{}", i)).and_then(|s| s.parse::<f64>().ok());
                let ea = self.properties.get(&format!("EA{}", i)).and_then(|s| s.parse::<f64>().ok());
                let r = self.properties.get(&format!("R{}", i)).and_then(|s| s.parse::<i32>().ok()).map(PcbCoord::from_raw);
                (cx, cy, sa, ea, r)
            } else {
                (None, None, None, None, None)
            };

            verts.push(PolygonVertex {
                kind,
                x: PcbCoord::from_raw(x),
                y: PcbCoord::from_raw(y),
                cx,
                cy,
                sa,
                ea,
                radius,
            });
            i += 1;
        }
        verts
    }
}
