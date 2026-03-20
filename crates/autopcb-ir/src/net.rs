//! Net (electrical connectivity) types.

use crate::handles::{ComponentId, NetId, PadId};
use crate::types::PointMm;

/// An electrical net connecting pads across the board.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct IrNet {
    pub id: NetId,
    pub name: String,
    /// Pins (pads) belonging to this net.
    pub pins: Vec<IrNetPin>,
    /// Number of distinct components this net touches.
    pub component_count: usize,
    /// Net class name this net belongs to, if any.
    pub net_class: Option<String>,
    /// The partner net in a differential pair (e.g. the negative net when this is the positive).
    pub diff_pair_partner: Option<NetId>,
}

/// A single pin (pad) within a net.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct IrNetPin {
    pub pad: PadId,
    pub component: ComponentId,
    pub position: PointMm,
}
