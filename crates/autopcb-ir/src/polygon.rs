//! Copper pour polygon representation.

use crate::handles::{NetId, PolygonId};
use crate::types::PointMm;

/// A copper pour polygon.
#[derive(Debug, Clone)]
pub struct IrPolygon {
    pub id: PolygonId,
    pub name: String,
    pub net: Option<NetId>,
    pub layer_name: String,
    pub vertices: Vec<PointMm>,
}
