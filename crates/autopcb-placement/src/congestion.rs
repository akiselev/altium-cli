//! Congestion oracle integration point for placement-router co-optimization.
//!
//! # Overview
//!
//! The [`CongestionOracle`] trait is the stable interface between the placement
//! engine and the router's congestion model.  It lets the SA cost function add
//! a routing-congestion penalty without depending on `autopcb-router` at
//! compile time.
//!
//! # Usage without the `routing` feature
//!
//! The trait is always available.  The placement code accepts
//! `Option<&dyn CongestionOracle>`.  When `None` is passed the congestion
//! term is zero and no routing dependency is needed.
//!
//! # Usage with the `routing` feature
//!
//! Enable `routing` in `autopcb-placement`'s feature list.  This activates a
//! blanket `impl CongestionOracle for autopcb_router::CongestionGrid`, so you
//! can pass a `&CongestionGrid` directly:
//!
//! ```ignore
//! let grid = autopcb_router::congestion_oracle(&ir, &config)?;
//! let penalty = grid.congestion_penalty_at(x_mm, y_mm);
//! ```
//!
//! # Integration in the SA cost function
//!
//! The SA [`crate::simulated_annealing::SAConfig`] already exposes
//! `congestion_weight: f64`.  When a [`CongestionOracle`] is available the SA
//! queries `congestion_penalty_at(comp_x, comp_y)` for every component that
//! has been moved, multiplies by `congestion_weight`, and adds the result to
//! the HPWL-based cost delta.
//!
//! The integration point lives in [`apply_external_congestion_penalty`].

// ---------------------------------------------------------------------------
// CongestionOracle trait
// ---------------------------------------------------------------------------

/// Trait for querying routing-congestion estimates at board positions.
///
/// Implemented by `autopcb_router::CongestionGrid` (when the `routing` feature
/// is enabled) and by any user-supplied struct.  An implementation should:
/// - Return `0.0` for out-of-bounds positions.
/// - Return a value ≥ 0.0; values > 1.0 indicate oversubscription.
/// - Be deterministic: identical inputs must produce identical outputs.
pub trait CongestionOracle {
    /// Congestion penalty at world position `(x_mm, y_mm)`.
    ///
    /// Typical use: multiply by `congestion_weight` and add to placement cost.
    fn congestion_penalty_at(&self, x_mm: f64, y_mm: f64) -> f64;
}

// ---------------------------------------------------------------------------
// Blanket impl for `autopcb_router::CongestionGrid` (behind `routing` feature)
// ---------------------------------------------------------------------------

#[cfg(feature = "routing")]
impl CongestionOracle for autopcb_router::CongestionGrid {
    fn congestion_penalty_at(&self, x_mm: f64, y_mm: f64) -> f64 {
        self.congestion_at(x_mm, y_mm)
    }
}

// ---------------------------------------------------------------------------
// apply_external_congestion_penalty
// ---------------------------------------------------------------------------

/// Compute the additional congestion penalty for a set of component positions.
///
/// `positions` is a slice of `(x_mm, y_mm)` world-space positions (one per
/// component, or just the moved component in an incremental evaluation).
/// Returns the sum of `oracle.congestion_penalty_at(x, y) * weight` over all
/// positions, or `0.0` if `oracle` is `None` or `weight` is zero.
///
/// This function is the single integration point between the SA cost function
/// and the external congestion model.  Callers do not need to check feature
/// flags or handle `None` oracles separately.
pub fn apply_external_congestion_penalty(
    oracle: Option<&dyn CongestionOracle>,
    positions: &[(f64, f64)],
    weight: f64,
) -> f64 {
    if weight == 0.0 {
        return 0.0;
    }
    let oracle = match oracle {
        Some(o) => o,
        None => return 0.0,
    };
    positions
        .iter()
        .map(|&(x, y)| oracle.congestion_penalty_at(x, y) * weight)
        .sum()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// A simple test oracle that returns the x coordinate as the penalty.
    struct LinearOracle;

    impl CongestionOracle for LinearOracle {
        fn congestion_penalty_at(&self, x_mm: f64, _y_mm: f64) -> f64 {
            x_mm.max(0.0)
        }
    }

    #[test]
    fn penalty_with_none_oracle_is_zero() {
        let result = apply_external_congestion_penalty(None, &[(10.0, 5.0)], 1.0);
        assert!((result - 0.0).abs() < f64::EPSILON, "None oracle must yield 0.0");
    }

    #[test]
    fn penalty_with_zero_weight_is_zero() {
        let oracle = LinearOracle;
        let result = apply_external_congestion_penalty(Some(&oracle), &[(5.0, 0.0)], 0.0);
        assert!((result - 0.0).abs() < f64::EPSILON, "weight=0 must yield 0.0");
    }

    #[test]
    fn penalty_sums_over_positions() {
        let oracle = LinearOracle;
        // positions with x = 1.0 and x = 2.0; weight = 1.0
        // expected penalty = 1.0 + 2.0 = 3.0
        let result = apply_external_congestion_penalty(
            Some(&oracle),
            &[(1.0, 0.0), (2.0, 0.0)],
            1.0,
        );
        assert!(
            (result - 3.0).abs() < f64::EPSILON,
            "expected 3.0, got {result}"
        );
    }

    #[test]
    fn penalty_scaled_by_weight() {
        let oracle = LinearOracle;
        // x=4.0, weight=2.5 → 4.0 × 2.5 = 10.0
        let result =
            apply_external_congestion_penalty(Some(&oracle), &[(4.0, 0.0)], 2.5);
        assert!(
            (result - 10.0).abs() < f64::EPSILON,
            "expected 10.0, got {result}"
        );
    }

    #[test]
    fn penalty_empty_positions_is_zero() {
        let oracle = LinearOracle;
        let result = apply_external_congestion_penalty(Some(&oracle), &[], 1.0);
        assert!((result - 0.0).abs() < f64::EPSILON, "empty positions must yield 0.0");
    }
}
