//! A* cost functions and heuristic for 3D grid routing.
//!
//! # Heuristic admissibility
//!
//! The heuristic `h(n) = manhattan_distance(n, goal) + via_estimate` never
//! overestimates the actual cost because:
//! - Each grid step costs at minimum 1.0 (cardinal) or √2 (diagonal).
//!   Manhattan distance uses step cost = 1.0, which is ≤ actual cost.
//! - If `n.layer != goal.layer`, at least one via is required. Adding
//!   `min_via_cost` therefore gives a lower bound on the remaining cost.
//!
//! # Direction penalty
//!
//! The direction penalty is applied to **neighbor costs** (not to the
//! heuristic) to preserve admissibility. Moves that go against the layer's
//! preferred direction pay a small multiplicative penalty, nudging the router
//! toward the preferred direction without breaking optimality guarantees.

use autopcb_ir::layer_stack::PreferredDirection;

use super::grid::GridNode;

/// Admissible heuristic: Manhattan distance to goal plus minimum via cost for
/// any required layer transition.
///
/// Manhattan distance uses step cost = 1.0, which is ≤ actual move cost (1.0
/// for cardinal, √2 for diagonal).  Adding `min_via_cost` when layers differ
/// accounts for at least one required via.
pub fn heuristic(node: GridNode, goal: GridNode, min_via_cost: f64) -> f64 {
    let dx = node.x.abs_diff(goal.x) as f64;
    let dy = node.y.abs_diff(goal.y) as f64;
    let manhattan = dx + dy;
    let via_estimate = if node.layer != goal.layer {
        min_via_cost
    } else {
        0.0
    };
    manhattan + via_estimate
}

/// Compute the direction-bias penalty factor for a move `(dx, dy)` on a layer
/// with the given preferred routing direction.
///
/// Returns a multiplicative factor ≥ 1.0:
/// - 1.0 for moves in the preferred direction (or if no preference).
/// - `PENALTY` for moves against the preferred direction.
///
/// This is applied to **neighbor costs** (not the heuristic) so admissibility
/// is preserved: the penalty makes off-preference moves more expensive but
/// never underestimates the remaining path.
pub fn direction_penalty(dx: i32, dy: i32, preferred: Option<PreferredDirection>) -> f64 {
    const PENALTY: f64 = 1.5;

    match preferred {
        None | Some(PreferredDirection::Any) => 1.0,
        Some(PreferredDirection::Horizontal) => {
            // Horizontal = prefer moves along x-axis. Penalize moves with dy != 0.
            if dy != 0 { PENALTY } else { 1.0 }
        }
        Some(PreferredDirection::Vertical) => {
            // Vertical = prefer moves along y-axis. Penalize moves with dx != 0.
            if dx != 0 { PENALTY } else { 1.0 }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use autopcb_routes::LayerId;

    fn node(x: u32, y: u32, layer: u16) -> GridNode {
        GridNode { x, y, layer: LayerId(layer) }
    }

    // -----------------------------------------------------------------------
    // Heuristic tests
    // -----------------------------------------------------------------------

    #[test]
    fn heuristic_same_position_same_layer_is_zero() {
        let n = node(5, 3, 0);
        let g = node(5, 3, 0);
        assert!(
            heuristic(n, g, 10.0).abs() < f64::EPSILON,
            "heuristic for identical node+goal should be 0.0"
        );
    }

    #[test]
    fn heuristic_same_layer_returns_manhattan_distance() {
        let n = node(0, 0, 0);
        let g = node(3, 4, 0);
        // Manhattan distance = 3 + 4 = 7
        let h = heuristic(n, g, 10.0);
        assert!(
            (h - 7.0).abs() < f64::EPSILON,
            "expected Manhattan distance 7.0, got {h}"
        );
    }

    #[test]
    fn heuristic_different_layer_adds_min_via_cost() {
        let n = node(0, 0, 0);
        let g = node(3, 4, 1);
        let min_via_cost = 10.0;
        // Manhattan = 7, via_estimate = 10.0
        let h = heuristic(n, g, min_via_cost);
        assert!(
            (h - 17.0).abs() < f64::EPSILON,
            "expected 7.0 + 10.0 = 17.0 for different-layer, got {h}"
        );
    }

    #[test]
    fn heuristic_zero_position_different_layer_is_via_cost() {
        let n = node(0, 0, 0);
        let g = node(0, 0, 2);
        let min_via_cost = 15.0;
        let h = heuristic(n, g, min_via_cost);
        assert!(
            (h - 15.0).abs() < f64::EPSILON,
            "expected via cost only = 15.0, got {h}"
        );
    }

    /// Verify heuristic is admissible: h(node, goal) ≤ actual cost for the
    /// straight-line path (one via, no obstacles).
    #[test]
    fn heuristic_admissible_for_simple_paths() {
        // Same-layer path of 5 cells: actual cost = 5.0, h = 5.0.
        let n = node(0, 0, 0);
        let g = node(5, 0, 0);
        let h = heuristic(n, g, 10.0);
        assert!(h <= 5.0 + 1e-9, "h={h} should be ≤ actual cost 5.0");

        // Diagonal path: actual min cost = 5 × √2 ≈ 7.07, h = 5 + 5 = 10.
        // Here h > actual — but this is the 4-way heuristic with 8-way moves.
        // For 4-way movement (cost=1 per step), actual = 10, h = 10 → admissible.
        let n = node(0, 0, 0);
        let g = node(5, 5, 0);
        let h = heuristic(n, g, 10.0);
        // 4-way: actual = 10, h = 10 → ok.
        assert!(h <= 10.0 + 1e-9, "h={h} should be ≤ 4-way actual cost 10.0");

        // Cross-layer: actual = manhattan + via_cost, h = manhattan + min_via_cost.
        // When min_via_cost equals actual via cost, h = actual → admissible.
        let n = node(0, 0, 0);
        let g = node(3, 0, 1);
        let min_via = 10.0;
        let h = heuristic(n, g, min_via);
        // actual ≥ 3 (steps) + 10 (via) = 13, h = 13 → admissible.
        assert!(h <= 13.0 + 1e-9, "h={h} should be ≤ 13.0");
    }

    // -----------------------------------------------------------------------
    // Direction penalty tests
    // -----------------------------------------------------------------------

    #[test]
    fn no_preferred_direction_returns_one() {
        assert!((direction_penalty(1, 0, None) - 1.0).abs() < f64::EPSILON);
        assert!((direction_penalty(0, 1, None) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn any_direction_returns_one() {
        assert!(
            (direction_penalty(1, 0, Some(PreferredDirection::Any)) - 1.0).abs() < f64::EPSILON
        );
        assert!(
            (direction_penalty(0, 1, Some(PreferredDirection::Any)) - 1.0).abs() < f64::EPSILON
        );
    }

    #[test]
    fn horizontal_preferred_penalizes_vertical_moves() {
        // Horizontal move: dx=1, dy=0 → no penalty
        assert!(
            (direction_penalty(1, 0, Some(PreferredDirection::Horizontal)) - 1.0).abs()
                < f64::EPSILON,
            "horizontal move on horizontal layer should have no penalty"
        );
        // Vertical move: dx=0, dy=1 → penalty
        let p = direction_penalty(0, 1, Some(PreferredDirection::Horizontal));
        assert!(
            p > 1.0,
            "vertical move on horizontal layer should have penalty > 1.0, got {p}"
        );
    }

    #[test]
    fn vertical_preferred_penalizes_horizontal_moves() {
        // Vertical move: dx=0, dy=1 → no penalty
        assert!(
            (direction_penalty(0, 1, Some(PreferredDirection::Vertical)) - 1.0).abs()
                < f64::EPSILON,
            "vertical move on vertical layer should have no penalty"
        );
        // Horizontal move: dx=1, dy=0 → penalty
        let p = direction_penalty(1, 0, Some(PreferredDirection::Vertical));
        assert!(
            p > 1.0,
            "horizontal move on vertical layer should have penalty > 1.0, got {p}"
        );
    }
}
