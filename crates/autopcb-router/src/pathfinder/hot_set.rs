//! Hot-set tracking: identifies the worst-offending nets for partial rip-up.
//!
//! Tracks nets that route through persistently oversubscribed cells so that
//! partial rip-up can target only those nets rather than ripping everything.

use std::collections::{HashMap, HashSet};

use autopcb_routes::NetId;

use crate::detailed::grid::PathSegment;

// ---------------------------------------------------------------------------
// HotSet
// ---------------------------------------------------------------------------

/// Set of nets that route through oversubscribed cells.
///
/// Built from the conflict list produced by `count_conflicts` and the current
/// solution paths. Used for partial rip-up: only hot-set nets are removed and
/// rerouted, leaving non-conflicting nets in place.
#[derive(Debug, Clone, Default)]
pub struct HotSet {
    net_ids: HashSet<NetId>,
}

impl HotSet {
    /// Build a `HotSet` from a list of oversubscribed cells and the current
    /// per-net path map.
    ///
    /// Any net whose path passes through at least one oversubscribed cell is
    /// added to the hot set.
    pub fn from_conflicts(
        conflicts: &[(u32, u32, u16)],
        net_paths: &HashMap<NetId, Vec<PathSegment>>,
    ) -> HotSet {
        // Build a fast lookup set for the conflicted cells.
        let conflict_set: HashSet<(u32, u32, u16)> = conflicts.iter().copied().collect();

        let mut hot = HashSet::new();

        for (&net_id, segments) in net_paths {
            'segments: for seg in segments {
                // Check start node.
                let start_cell = (seg.start.x, seg.start.y, seg.start.layer.raw());
                if conflict_set.contains(&start_cell) {
                    hot.insert(net_id);
                    break 'segments;
                }
                // Check end node.
                let end_cell = (seg.end.x, seg.end.y, seg.end.layer.raw());
                if conflict_set.contains(&end_cell) {
                    hot.insert(net_id);
                    break 'segments;
                }
            }
        }

        HotSet { net_ids: hot }
    }

    /// Returns `true` if `net_id` is in the hot set.
    pub fn contains(&self, net_id: NetId) -> bool {
        self.net_ids.contains(&net_id)
    }

    /// Number of nets in the hot set.
    pub fn len(&self) -> usize {
        self.net_ids.len()
    }

    /// Returns `true` if the hot set is empty.
    pub fn is_empty(&self) -> bool {
        self.net_ids.is_empty()
    }

    /// Iterate over the net IDs in the hot set.
    pub fn iter(&self) -> impl Iterator<Item = NetId> + '_ {
        self.net_ids.iter().copied()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    use autopcb_routes::{LayerId, NetId};

    use crate::detailed::grid::{GridNode, PathSegment};

    fn seg(x0: u32, y0: u32, x1: u32, y1: u32, layer: u16) -> PathSegment {
        PathSegment {
            start: GridNode { x: x0, y: y0, layer: LayerId(layer) },
            end: GridNode { x: x1, y: y1, layer: LayerId(layer) },
        }
    }

    fn make_paths(entries: &[(u32, Vec<PathSegment>)]) -> HashMap<NetId, Vec<PathSegment>> {
        entries.iter().map(|(id, segs)| (NetId(*id), segs.clone())).collect()
    }

    #[test]
    fn empty_conflicts_produces_empty_hot_set() {
        let paths = make_paths(&[(0, vec![seg(0, 0, 1, 0, 0)])]);
        let hot = HotSet::from_conflicts(&[], &paths);
        assert!(hot.is_empty());
        assert!(!hot.contains(NetId(0)));
    }

    #[test]
    fn net_through_conflict_cell_is_hot() {
        let paths = make_paths(&[
            (0, vec![seg(3, 3, 4, 3, 0)]), // passes through (3,3,0) and (4,3,0)
            (1, vec![seg(0, 0, 1, 0, 0)]), // far away
        ]);
        let conflicts = vec![(3, 3, 0_u16)];
        let hot = HotSet::from_conflicts(&conflicts, &paths);
        assert!(hot.contains(NetId(0)), "net 0 should be in hot set");
        assert!(!hot.contains(NetId(1)), "net 1 should not be in hot set");
    }

    #[test]
    fn net_not_through_conflict_is_not_hot() {
        let paths = make_paths(&[(0, vec![seg(0, 0, 1, 0, 0)])]);
        let conflicts = vec![(5, 5, 0_u16)];
        let hot = HotSet::from_conflicts(&conflicts, &paths);
        assert!(!hot.contains(NetId(0)));
    }

    #[test]
    fn multiple_nets_some_hot() {
        let paths = make_paths(&[
            (0, vec![seg(5, 5, 6, 5, 0)]),
            (1, vec![seg(5, 5, 5, 6, 0)]),
            (2, vec![seg(0, 0, 1, 0, 0)]),
        ]);
        let conflicts = vec![(5, 5, 0_u16)];
        let hot = HotSet::from_conflicts(&conflicts, &paths);
        assert!(hot.contains(NetId(0)));
        assert!(hot.contains(NetId(1)));
        assert!(!hot.contains(NetId(2)));
        assert_eq!(hot.len(), 2);
    }

    #[test]
    fn conflict_on_end_node_also_detected() {
        let paths = make_paths(&[(0, vec![seg(2, 2, 9, 9, 0)])]);
        // Cell (9,9,0) is the end node of the segment.
        let conflicts = vec![(9, 9, 0_u16)];
        let hot = HotSet::from_conflicts(&conflicts, &paths);
        assert!(hot.contains(NetId(0)), "end-node conflict should mark net as hot");
    }
}
