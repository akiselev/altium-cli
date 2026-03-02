//! Net (electrical connectivity) types.

use crate::handles::{ComponentId, NetId, PadId};
use crate::types::PointMm;

/// An electrical net connecting pads across the board.
#[derive(Debug, Clone)]
pub struct IrNet {
    pub id: NetId,
    pub name: String,
    /// Pins (pads) belonging to this net.
    pub pins: Vec<IrNetPin>,
    /// Number of distinct components this net touches.
    pub component_count: usize,
}

/// A single pin (pad) within a net.
#[derive(Debug, Clone)]
pub struct IrNetPin {
    pub pad: PadId,
    pub component: ComponentId,
    pub position: PointMm,
}
