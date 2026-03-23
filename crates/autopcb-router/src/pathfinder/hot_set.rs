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

    /// Build a hot set with an adaptive size cap.
    ///
    /// Cap = `(3 * oversubscribed_count).clamp(64, 150)`.
    /// When more nets touch conflicts than the cap, prioritize by the number
    /// of conflict cells each net touches (highest first).
    pub fn from_conflicts_adaptive(
        conflicts: &[(u32, u32, u16)],
        net_paths: &HashMap<NetId, Vec<PathSegment>>,
    ) -> HotSet {
        let cap = (3 * conflicts.len()).clamp(64, 150);

        // Score each net by the number of conflict cells it touches.
        let conflict_set: HashSet<(u32, u32, u16)> = conflicts.iter().copied().collect();
        let mut net_scores: HashMap<NetId, usize> = HashMap::new();

        for (&net_id, segments) in net_paths {
            let mut score = 0usize;
            for seg in segments {
                if conflict_set.contains(&(seg.start.x, seg.start.y, seg.start.layer.raw())) {
                    score += 1;
                }
                if conflict_set.contains(&(seg.end.x, seg.end.y, seg.end.layer.raw())) {
                    score += 1;
                }
            }
            if score > 0 {
                net_scores.insert(net_id, score);
            }
        }

        // If within cap, return all
        if net_scores.len() <= cap {
            return HotSet {
                net_ids: net_scores.keys().copied().collect(),
            };
        }

        // Sort by score descending, take top `cap`
        let mut scored: Vec<_> = net_scores.into_iter().collect();
        scored.sort_by(|a, b| b.1.cmp(&a.1));
        scored.truncate(cap);

        HotSet {
            net_ids: scored.into_iter().map(|(id, _)| id).collect(),
        }
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

    // ---------------------------------------------------------------------------
    // from_conflicts_adaptive tests
    // ---------------------------------------------------------------------------

    #[test]
    fn adaptive_under_cap_returns_all_nets() {
        // With 3 conflict cells, cap = clamp(9, 64, 150) = 64.
        // We have 3 nets touching conflicts — all should be in the hot set.
        let conflicts = vec![(0, 0, 0_u16), (1, 0, 0_u16), (2, 0, 0_u16)];
        let paths = make_paths(&[
            (0, vec![seg(0, 0, 3, 0, 0)]), // touches (0,0,0) start
            (1, vec![seg(1, 0, 3, 0, 0)]), // touches (1,0,0) start
            (2, vec![seg(2, 0, 3, 0, 0)]), // touches (2,0,0) start
        ]);
        let hot = HotSet::from_conflicts_adaptive(&conflicts, &paths);
        assert!(hot.contains(NetId(0)), "net 0 should be in adaptive hot set");
        assert!(hot.contains(NetId(1)), "net 1 should be in adaptive hot set");
        assert!(hot.contains(NetId(2)), "net 2 should be in adaptive hot set");
        assert_eq!(hot.len(), 3);
    }

    #[test]
    fn adaptive_empty_conflicts_returns_empty() {
        let paths = make_paths(&[(0, vec![seg(0, 0, 1, 0, 0)])]);
        let hot = HotSet::from_conflicts_adaptive(&[], &paths);
        assert!(hot.is_empty());
    }

    #[test]
    fn adaptive_net_not_through_conflict_excluded() {
        let conflicts = vec![(5, 5, 0_u16)];
        let paths = make_paths(&[
            (0, vec![seg(0, 0, 1, 0, 0)]), // does not touch (5,5,0)
        ]);
        let hot = HotSet::from_conflicts_adaptive(&conflicts, &paths);
        assert!(!hot.contains(NetId(0)));
        assert!(hot.is_empty());
    }

    #[test]
    fn adaptive_truncates_to_cap_when_over() {
        // With 1 conflict cell, cap = clamp(3, 64, 150) = 64.
        // We create 70 nets all touching the same conflict cell.
        // All 70 are under cap=64? No: 70 > 64, so cap=64 nets are kept.
        // Wait: 3 * 1 = 3, clamp(3,64,150) = 64. 70 > 64, so truncate to 64.
        let conflicts = vec![(0, 0, 0_u16)];
        let entries: Vec<(u32, Vec<PathSegment>)> = (0u32..70)
            .map(|i| (i, vec![seg(0, 0, i + 1, 0, 0)]))
            .collect();
        let paths = make_paths(&entries);
        let hot = HotSet::from_conflicts_adaptive(&conflicts, &paths);
        assert_eq!(hot.len(), 64, "adaptive hot set should be capped at 64");
    }

    #[test]
    fn adaptive_higher_score_nets_preferred_when_truncating() {
        // With 22 conflict cells, cap = clamp(66, 64, 150) = 66.
        // Net 0 touches many conflict cells (score = 10).
        // Nets 1-100 each touch exactly 1 conflict cell.
        // 101 nets total, cap = 66 (since 3*22=66, but we need > 66 nets).
        // Actually let's use fewer conflict cells to force cap=64 and check
        // that high-score nets are always included.

        // 1 conflict cell, cap = 64.
        // Net 0 has score 2 (both start and end hit the conflict).
        // Nets 1-70 each have score 1.
        // With 71 nets > cap 64, net 0 must survive the truncation.
        let conflicts = vec![(0, 0, 0_u16)];
        let mut entries: Vec<(u32, Vec<PathSegment>)> = Vec::new();
        // Net 0: both endpoints hit (0,0,0) — score 2
        entries.push((0, vec![seg(0, 0, 0, 0, 0)]));
        // Nets 1..71: start at (0,0,0) — score 1 each
        for i in 1u32..71 {
            entries.push((i, vec![seg(0, 0, i + 1, 0, 0)]));
        }
        let paths = make_paths(&entries);
        let hot = HotSet::from_conflicts_adaptive(&conflicts, &paths);
        assert_eq!(hot.len(), 64, "adaptive hot set should be capped at 64");
        assert!(hot.contains(NetId(0)), "net 0 with highest score must be in hot set");
    }
}
