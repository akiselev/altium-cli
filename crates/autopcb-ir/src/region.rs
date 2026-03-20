//! Region primitives in the IR (copper pours, solder mask openings, etc.).

use crate::handles::{NetId, RegionId};
use crate::types::PointMm;

/// A PCB region (copper pour, solder mask opening, paste mask, board cutout).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct IrRegion {
    pub id: RegionId,
    /// The kind of region.
    pub kind: IrRegionKind,
    /// Outer boundary in mm (tessellated, no arcs).
    pub outline: Vec<PointMm>,
    /// Interior holes, each tessellated.
    pub holes: Vec<Vec<PointMm>>,
    /// Layer display name.
    pub layer_name: String,
    /// Net assignment, if any.
    pub net: Option<NetId>,
    /// Whether this region acts as a keepout.
    pub is_keepout: bool,
}

/// Classification of a region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum IrRegionKind {
    CopperPour,
    SolderMask,
    PasteMask,
    BoardCutout,
    Other,
}
