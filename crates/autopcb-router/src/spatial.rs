//! Spatial indexing over fixed and dynamic obstacles.
//!
//! Wraps `rstar::RTree<ObstacleEntry>` and provides clearance queries used
//! during A* pathfinding.
//!
//! # Obstacle classification
//!
//! - **Fixed**: pads, keepouts, board edge.  Never moved by the router.
//! - **Pre-routed**: locked tracks and vias from a prior routing pass.
//!   Must be preserved (router may not rip them up).
//! - **Solution occupancy**: tracks/vias placed in the current invocation.
//!   Added to the R-tree as each net is routed; mutable.
//!
//! The `ObstacleEntry` enum models all three categories in a single tree to
//! keep query code simple.

use autopcb_ir::handles::LayerId as IrLayerId;
use autopcb_routes::{LayerId, NetId};
use rstar::{RTreeObject, AABB};

// ---------------------------------------------------------------------------
// ObstacleEntry
// ---------------------------------------------------------------------------

/// Axis-aligned bounding rectangle used as the R-tree envelope (2-D, in mm).
type Envelope = AABB<[f64; 2]>;

/// A single obstacle stored in the spatial index.
///
/// Each variant stores enough geometry and metadata for clearance queries and
/// same-net pass-through logic.
#[derive(Debug, Clone)]
pub enum ObstacleEntry {
    /// A component pad.
    Pad {
        /// Bounding box of the pad (inflated by annular ring / pad size).
        bounds: [f64; 4],
        /// Net this pad belongs to (`None` = unconnected).
        net_id: Option<NetId>,
        /// Layer (routes LayerId). Through-hole pads have one entry per layer.
        layer: LayerId,
    },
    /// A keepout zone polygon.
    Keepout {
        /// Bounding box of the keepout region.
        bounds: [f64; 4],
        /// Layer restriction (`None` = all layers).
        layer: Option<LayerId>,
    },
    /// The board edge (cells outside the outline are out-of-bounds).
    BoardEdge {
        /// Bounding box of a single edge segment.
        bounds: [f64; 4],
    },
    /// A track from a previous routing pass (locked, must not be moved).
    PreRoutedTrack {
        /// Bounding box of the track segment (inflated by half-width).
        bounds: [f64; 4],
        /// Net this track belongs to.
        net_id: Option<NetId>,
        /// Layer the track is on.
        layer: LayerId,
    },
    /// A via from a previous routing pass (locked, must not be moved).
    PreRoutedVia {
        /// Bounding box of the via (annular ring).
        bounds: [f64; 4],
        /// Net this via belongs to.
        net_id: Option<NetId>,
        /// Layer the via spans from.
        from_layer: LayerId,
        /// Layer the via spans to.
        to_layer: LayerId,
    },
}

impl ObstacleEntry {
    /// Bounding box as `[min_x, min_y, max_x, max_y]`.
    pub(crate) fn raw_bounds(&self) -> [f64; 4] {
        match self {
            ObstacleEntry::Pad { bounds, .. }
            | ObstacleEntry::Keepout { bounds, .. }
            | ObstacleEntry::BoardEdge { bounds }
            | ObstacleEntry::PreRoutedTrack { bounds, .. }
            | ObstacleEntry::PreRoutedVia { bounds, .. } => *bounds,
        }
    }

    /// Net ID of this obstacle, if any.
    pub fn net_id(&self) -> Option<NetId> {
        match self {
            ObstacleEntry::Pad { net_id, .. } => *net_id,
            ObstacleEntry::PreRoutedTrack { net_id, .. } => *net_id,
            ObstacleEntry::PreRoutedVia { net_id, .. } => *net_id,
            ObstacleEntry::Keepout { .. } | ObstacleEntry::BoardEdge { .. } => None,
        }
    }

    /// Primary layer of this obstacle.  For vias returns the `from_layer`.
    /// For keepouts without a layer restriction returns `LayerId(0)` (sentinel).
    pub fn layer(&self) -> Option<LayerId> {
        match self {
            ObstacleEntry::Pad { layer, .. } => Some(*layer),
            ObstacleEntry::PreRoutedTrack { layer, .. } => Some(*layer),
            ObstacleEntry::PreRoutedVia { from_layer, .. } => Some(*from_layer),
            ObstacleEntry::Keepout { layer, .. } => *layer,
            ObstacleEntry::BoardEdge { .. } => None,
        }
    }

    // ---------------------------------------------------------------------------
    // Constructors
    // ---------------------------------------------------------------------------

    /// Pad obstacle from a bounding box in mm.
    pub fn pad(
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        net_id: Option<NetId>,
        ir_layer: IrLayerId,
    ) -> Self {
        debug_assert!(
            ir_layer.raw() <= u16::MAX as u32,
            "LayerId({}) overflows u16",
            ir_layer.raw()
        );
        ObstacleEntry::Pad {
            bounds: [min_x, min_y, max_x, max_y],
            net_id,
            layer: LayerId(ir_layer.raw() as u16),
        }
    }

    /// Keepout obstacle with optional layer restriction.
    pub fn keepout(
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        layer: Option<LayerId>,
    ) -> Self {
        ObstacleEntry::Keepout {
            bounds: [min_x, min_y, max_x, max_y],
            layer,
        }
    }

    /// Board edge segment obstacle.
    pub fn board_edge(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        ObstacleEntry::BoardEdge {
            bounds: [min_x, min_y, max_x, max_y],
        }
    }

    /// Pre-routed track obstacle from a bounding box in mm.
    pub fn pre_routed_track(
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        net_id: Option<NetId>,
        ir_layer: IrLayerId,
    ) -> Self {
        debug_assert!(
            ir_layer.raw() <= u16::MAX as u32,
            "LayerId({}) overflows u16",
            ir_layer.raw()
        );
        ObstacleEntry::PreRoutedTrack {
            bounds: [min_x, min_y, max_x, max_y],
            net_id,
            layer: LayerId(ir_layer.raw() as u16),
        }
    }

    /// Pre-routed via obstacle from a bounding box in mm.
    pub fn pre_routed_via(
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        net_id: Option<NetId>,
        ir_from: IrLayerId,
        ir_to: IrLayerId,
    ) -> Self {
        debug_assert!(
            ir_from.raw() <= u16::MAX as u32,
            "LayerId({}) overflows u16",
            ir_from.raw()
        );
        debug_assert!(
            ir_to.raw() <= u16::MAX as u32,
            "LayerId({}) overflows u16",
            ir_to.raw()
        );
        ObstacleEntry::PreRoutedVia {
            bounds: [min_x, min_y, max_x, max_y],
            net_id,
            from_layer: LayerId(ir_from.raw() as u16),
            to_layer: LayerId(ir_to.raw() as u16),
        }
    }
}

impl RTreeObject for ObstacleEntry {
    type Envelope = Envelope;

    fn envelope(&self) -> Self::Envelope {
        let [min_x, min_y, max_x, max_y] = self.raw_bounds();
        AABB::from_corners([min_x, min_y], [max_x, max_y])
    }
}

// ---------------------------------------------------------------------------
// SpatialIndex
// ---------------------------------------------------------------------------

/// Spatial index over `ObstacleEntry` values.
///
/// Wraps an `rstar::RTree` for O(log n) region queries.  The tree is built
/// in bulk from a `Vec<ObstacleEntry>` (see [`SpatialIndex::build`]) and is
/// immutable after construction.  Dynamic (solution-occupancy) obstacles are
/// added incrementally via [`SpatialIndex::insert`].
pub struct SpatialIndex {
    tree: rstar::RTree<ObstacleEntry>,
}

impl SpatialIndex {
    /// Build a spatial index from a collection of obstacles in bulk.
    ///
    /// Bulk loading is faster than sequential insertion for large obstacle sets.
    pub fn build(obstacles: Vec<ObstacleEntry>) -> Self {
        SpatialIndex {
            tree: rstar::RTree::bulk_load(obstacles),
        }
    }

    /// Insert a single obstacle (used for solution-occupancy updates).
    pub fn insert(&mut self, entry: ObstacleEntry) {
        self.tree.insert(entry);
    }

    /// Return all obstacles whose bounding boxes intersect `aabb`
    /// (given as `[min_x, min_y, max_x, max_y]`).
    pub fn query_rect(&self, aabb: [f64; 4]) -> Vec<&ObstacleEntry> {
        let [min_x, min_y, max_x, max_y] = aabb;
        let envelope = AABB::from_corners([min_x, min_y], [max_x, max_y]);
        self.tree
            .locate_in_envelope_intersecting(&envelope)
            .collect()
    }

    /// Return all obstacles within `clearance` mm of the axis-aligned bounding
    /// box of the segment `(start_x, start_y) → (end_x, end_y)` on `layer`.
    ///
    /// The bounding box is expanded by `clearance` on all sides, so any
    /// obstacle that could be closer than `clearance` to the segment will be
    /// returned.  The caller is responsible for precise geometric distance
    /// checks if needed.
    pub fn clearance_query(
        &self,
        layer: LayerId,
        start_x: f64,
        start_y: f64,
        end_x: f64,
        end_y: f64,
        clearance: f64,
    ) -> Vec<&ObstacleEntry> {
        let min_x = start_x.min(end_x) - clearance;
        let min_y = start_y.min(end_y) - clearance;
        let max_x = start_x.max(end_x) + clearance;
        let max_y = start_y.max(end_y) + clearance;
        self.query_rect([min_x, min_y, max_x, max_y])
            .into_iter()
            .filter(|e| {
                // Keep obstacles on the queried layer, plus layer-agnostic
                // obstacles (BoardEdge, keepouts with no layer restriction).
                e.layer().map_or(true, |l| l == layer)
            })
            .collect()
    }

    /// Number of obstacles in the index.
    pub fn len(&self) -> usize {
        self.tree.size()
    }

    /// Returns `true` if the index contains no obstacles.
    pub fn is_empty(&self) -> bool {
        self.tree.size() == 0
    }
}

impl std::fmt::Debug for SpatialIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpatialIndex")
            .field("obstacle_count", &self.tree.size())
            .finish()
    }
}
