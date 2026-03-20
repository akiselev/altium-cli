//! Rip-up logic: full rip-up and per-net removal.
//!
//! Full rip-up removes all net paths each iteration (default PathFinder mode).
//! Per-net removal is used by hot-set partial rip-up to target only the worst
//! offenders.

use std::collections::HashMap;

use autopcb_routes::NetId;

use crate::detailed::grid::PathSegment;
use crate::workspace::GridConfig;

// ---------------------------------------------------------------------------
// Rip-up helpers
// ---------------------------------------------------------------------------

/// Remove all per-net paths, preparing for a fresh full reroute.
pub fn rip_up_all(solution_paths: &mut HashMap<NetId, Vec<PathSegment>>) {
    solution_paths.clear();
}

/// Remove the path for a single net, leaving other nets untouched.
pub fn rip_up_net(solution_paths: &mut HashMap<NetId, Vec<PathSegment>>, net_id: NetId) {
    solution_paths.remove(&net_id);
}

// ---------------------------------------------------------------------------
// Conflict counting
// ---------------------------------------------------------------------------

/// Count oversubscribed grid cells in the current solution paths.
///
/// A cell `(x, y, layer)` is oversubscribed when two or more distinct nets
/// occupy it simultaneously.
///
/// Returns `(conflict_count, oversubscribed_cells)` where each element of the
/// list is `(x, y, layer_raw)` for a cell with 2+ occupants.
pub fn count_conflicts(
    solution_paths: &HashMap<NetId, Vec<PathSegment>>,
    grid: &GridConfig,
    layer_count: usize,
) -> (u32, Vec<(u32, u32, u16)>) {
    // Map each grid cell to the set of nets that currently occupy it.
    // Key: (x, y, layer_raw); value: list of distinct net IDs.
    let mut cell_occupants: HashMap<(u32, u32, u16), Vec<NetId>> = HashMap::new();

    for (&net_id, segments) in solution_paths {
        for seg in segments {
            // Skip out-of-bounds nodes defensively.
            if !grid.in_bounds(seg.start.x, seg.start.y) {
                continue;
            }
            if seg.start.layer.raw() as usize >= layer_count {
                continue;
            }

            let cell = (seg.start.x, seg.start.y, seg.start.layer.raw());
            let occupants = cell_occupants.entry(cell).or_default();
            // Only add this net if not already recorded for this cell.
            if !occupants.contains(&net_id) {
                occupants.push(net_id);
            }

            // Also account for the end node of each segment.
            if !grid.in_bounds(seg.end.x, seg.end.y) {
                continue;
            }
            if seg.end.layer.raw() as usize >= layer_count {
                continue;
            }

            let end_cell = (seg.end.x, seg.end.y, seg.end.layer.raw());
            let end_occupants = cell_occupants.entry(end_cell).or_default();
            if !end_occupants.contains(&net_id) {
                end_occupants.push(net_id);
            }
        }
    }

    // Collect oversubscribed cells (2+ distinct nets).
    let mut oversubscribed: Vec<(u32, u32, u16)> = cell_occupants
        .into_iter()
        .filter(|(_, occupants)| occupants.len() >= 2)
        .map(|(cell, _)| cell)
        .collect();

    // Sort for determinism.
    oversubscribed.sort_unstable();

    let conflict_count = oversubscribed.len() as u32;
    (conflict_count, oversubscribed)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    use autopcb_ir::types::PointMm;
    use autopcb_routes::{LayerId, NetId};

    use crate::detailed::grid::{GridNode, PathSegment};
    use crate::workspace::GridConfig;

    fn make_grid() -> GridConfig {
        GridConfig {
            resolution_mm: 1.0,
            width_cells: 20,
            height_cells: 20,
            origin: PointMm::new(0.0, 0.0),
        }
    }

    fn seg(x0: u32, y0: u32, x1: u32, y1: u32, layer: u16) -> PathSegment {
        PathSegment {
            start: GridNode { x: x0, y: y0, layer: LayerId(layer) },
            end: GridNode { x: x1, y: y1, layer: LayerId(layer) },
        }
    }

    fn make_paths(
        entries: &[(u32, Vec<PathSegment>)],
    ) -> HashMap<NetId, Vec<PathSegment>> {
        entries.iter().map(|(id, segs)| (NetId(*id), segs.clone())).collect()
    }

    // --- rip_up_all ---------------------------------------------------------

    #[test]
    fn rip_up_all_clears_all_paths() {
        let mut paths = make_paths(&[
            (0, vec![seg(0, 0, 1, 0, 0)]),
            (1, vec![seg(5, 0, 6, 0, 0)]),
        ]);
        rip_up_all(&mut paths);
        assert!(paths.is_empty(), "all paths should be cleared");
    }

    #[test]
    fn rip_up_all_on_empty_is_noop() {
        let mut paths: HashMap<NetId, Vec<PathSegment>> = HashMap::new();
        rip_up_all(&mut paths);
        assert!(paths.is_empty());
    }

    // --- rip_up_net ---------------------------------------------------------

    #[test]
    fn rip_up_net_removes_only_specified_net() {
        let mut paths = make_paths(&[
            (0, vec![seg(0, 0, 1, 0, 0)]),
            (1, vec![seg(5, 0, 6, 0, 0)]),
        ]);
        rip_up_net(&mut paths, NetId(0));
        assert!(!paths.contains_key(&NetId(0)), "net 0 should be removed");
        assert!(paths.contains_key(&NetId(1)), "net 1 should remain");
    }

    #[test]
    fn rip_up_net_nonexistent_is_noop() {
        let mut paths = make_paths(&[(0, vec![seg(0, 0, 1, 0, 0)])]);
        rip_up_net(&mut paths, NetId(99));
        assert!(paths.contains_key(&NetId(0)), "existing net should be unaffected");
    }

    // --- count_conflicts ----------------------------------------------------

    #[test]
    fn two_non_overlapping_nets_produce_no_conflicts() {
        let paths = make_paths(&[
            (0, vec![seg(0, 0, 1, 0, 0), seg(1, 0, 2, 0, 0)]),
            (1, vec![seg(5, 0, 6, 0, 0), seg(6, 0, 7, 0, 0)]),
        ]);
        let grid = make_grid();
        let (count, cells) = count_conflicts(&paths, &grid, 2);
        assert_eq!(count, 0, "non-overlapping nets should have 0 conflicts");
        assert!(cells.is_empty());
    }

    #[test]
    fn two_overlapping_nets_produce_conflicts() {
        // Both nets pass through cell (3, 0, 0).
        let paths = make_paths(&[
            (0, vec![seg(2, 0, 3, 0, 0), seg(3, 0, 4, 0, 0)]),
            (1, vec![seg(1, 0, 3, 0, 0), seg(3, 0, 5, 0, 0)]),
        ]);
        let grid = make_grid();
        let (count, cells) = count_conflicts(&paths, &grid, 2);
        assert!(count > 0, "overlapping nets should produce conflicts");
        // Cell (3, 0, 0) should appear in oversubscribed list.
        assert!(
            cells.contains(&(3, 0, 0)),
            "cell (3,0,layer0) should be flagged as conflicted"
        );
    }

    #[test]
    fn same_net_does_not_conflict_with_itself() {
        // A net that visits the same cell twice (e.g. backtrack) should not
        // count as a conflict.
        let paths = make_paths(&[(0, vec![seg(3, 0, 3, 1, 0), seg(3, 1, 3, 0, 0)])]);
        let grid = make_grid();
        let (count, _) = count_conflicts(&paths, &grid, 2);
        assert_eq!(count, 0, "a single net revisiting a cell is not a conflict");
    }

    #[test]
    fn empty_paths_produce_no_conflicts() {
        let paths: HashMap<NetId, Vec<PathSegment>> = HashMap::new();
        let grid = make_grid();
        let (count, cells) = count_conflicts(&paths, &grid, 2);
        assert_eq!(count, 0);
        assert!(cells.is_empty());
    }

    #[test]
    fn different_layers_do_not_conflict() {
        // Same x,y but different layer should not be a conflict.
        let paths = make_paths(&[
            (0, vec![seg(5, 5, 6, 5, 0)]),
            (1, vec![seg(5, 5, 6, 5, 1)]),
        ]);
        let grid = make_grid();
        let (count, _) = count_conflicts(&paths, &grid, 2);
        assert_eq!(
            count, 0,
            "nets on different layers at the same (x,y) should not conflict"
        );
    }
}
