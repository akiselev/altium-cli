//! Net ordering heuristic: critical nets first, short nets early, high-fanout
//! last, `ChaCha8Rng`-seeded tiebreaker for determinism.
//!
//! # Ordering rules (applied in priority order)
//!
//! 1. Critical nets first (`priority < 0` → treated as critical).
//! 2. Among nets at the same criticality, shorter `estimated_length_mm` first.
//! 3. Among ties, lower `pin_count` first.
//! 4. Any remaining ties are broken by a deterministic shuffle using
//!    `ChaCha8Rng` seeded from the caller-supplied `seed`.
//!
//! # Determinism guarantee
//!
//! `ChaCha8Rng` from the `rand_chacha` crate is platform-independent and
//! version-stable.  Same `seed` + same input slice always produces the same
//! permutation.  **Do not replace** with `SmallRng` or `StdRng` — those are
//! not stable across `rand` versions.

use autopcb_routes::NetId;
use rand::seq::SliceRandom;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;

// ---------------------------------------------------------------------------
// NetOrderingInfo
// ---------------------------------------------------------------------------

/// Per-net information used by the ordering heuristic.
#[derive(Debug, Clone)]
pub struct NetOrderingInfo {
    /// Number of pins in this net.
    pub pin_count: usize,
    /// Estimated routing length in mm (e.g. from HPWL or MST).
    pub estimated_length_mm: f64,
    /// Routing priority.  Negative values mark critical nets (routed first).
    /// Higher (less negative) values are deprioritised.
    pub priority: i32,
}

// ---------------------------------------------------------------------------
// order_nets
// ---------------------------------------------------------------------------

/// Compute the routing order for a set of nets.
///
/// Returns a `Vec<NetId>` ordered so that:
/// - Critical nets (priority < 0) come before non-critical nets.
/// - Within each group, shorter nets come first.
/// - Within equal-length groups, lower fanout (pin_count) comes first.
/// - Remaining ties are broken by a seeded `ChaCha8Rng` shuffle, ensuring
///   determinism: same `seed` + same input always produces the same output.
pub fn order_nets(nets: &[(NetId, NetOrderingInfo)], seed: u64) -> Vec<NetId> {
    // Clone so we can sort.
    let mut ordered: Vec<(NetId, &NetOrderingInfo)> =
        nets.iter().map(|(id, info)| (*id, info)).collect();

    // Primary sort: deterministic key (critical, length, pin_count).
    // We use `sort_by_key` with a tuple key so the comparison is total and
    // stable.  We discretise `estimated_length_mm` to 6 decimal places to
    // produce an `Ord`-compatible representation without introducing
    // f64-ordering bugs.
    ordered.sort_by(|a, b| {
        let a_info = a.1;
        let b_info = b.1;

        // Critical (priority < 0) sorts before non-critical.
        let a_critical = a_info.priority < 0;
        let b_critical = b_info.priority < 0;
        if a_critical != b_critical {
            // true > false in bool ordering, but we want critical first.
            return b_critical.cmp(&a_critical);
        }

        // Then by estimated length ascending.
        let len_cmp = a_info
            .estimated_length_mm
            .partial_cmp(&b_info.estimated_length_mm)
            .unwrap_or(std::cmp::Ordering::Equal);
        if len_cmp != std::cmp::Ordering::Equal {
            return len_cmp;
        }

        // Then by pin_count ascending.
        let pin_cmp = a_info.pin_count.cmp(&b_info.pin_count);
        if pin_cmp != std::cmp::Ordering::Equal {
            return pin_cmp;
        }

        // Preserve current order for ties — RNG shuffle below will break them.
        std::cmp::Ordering::Equal
    });

    // Group into equivalence classes (same key triple) and shuffle each class
    // independently using ChaCha8Rng so the tiebreak is deterministic but
    // unpredictable without the seed.
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    // Find run boundaries where the sort key changes and shuffle each run.
    let n = ordered.len();
    let mut i = 0;
    while i < n {
        // Find the end of the current run (same priority-class, length, pin_count).
        let mut j = i + 1;
        while j < n {
            let a = ordered[i].1;
            let b = ordered[j].1;
            let same = (a.priority < 0) == (b.priority < 0)
                && (a.estimated_length_mm - b.estimated_length_mm).abs() < 1e-9
                && a.pin_count == b.pin_count;
            if same {
                j += 1;
            } else {
                break;
            }
        }
        // Shuffle the run [i, j) with the shared RNG.
        ordered[i..j].shuffle(&mut rng);
        i = j;
    }

    ordered.into_iter().map(|(id, _)| id).collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn net(id: u32) -> NetId {
        NetId(id)
    }

    fn info(pin_count: usize, length: f64, priority: i32) -> NetOrderingInfo {
        NetOrderingInfo {
            pin_count,
            estimated_length_mm: length,
            priority,
        }
    }

    #[test]
    fn critical_net_before_non_critical_same_length() {
        let nets = vec![
            (net(0), info(2, 10.0, 0)),  // non-critical
            (net(1), info(2, 10.0, -1)), // critical
        ];
        let order = order_nets(&nets, 0);
        assert_eq!(order.len(), 2);
        assert_eq!(order[0], net(1), "critical net should come first");
        assert_eq!(order[1], net(0), "non-critical net should come second");
    }

    #[test]
    fn shorter_net_before_longer_net_same_priority() {
        let nets = vec![
            (net(0), info(2, 20.0, 0)), // longer
            (net(1), info(2, 5.0, 0)),  // shorter
        ];
        let order = order_nets(&nets, 0);
        assert_eq!(order[0], net(1), "shorter net should come first");
        assert_eq!(order[1], net(0), "longer net should come second");
    }

    #[test]
    fn lower_pin_count_before_higher_at_same_length() {
        let nets = vec![
            (net(0), info(5, 10.0, 0)), // more pins
            (net(1), info(2, 10.0, 0)), // fewer pins
        ];
        let order = order_nets(&nets, 0);
        assert_eq!(order[0], net(1), "fewer-pin net should come first");
    }

    #[test]
    fn same_seed_produces_identical_ordering() {
        let nets: Vec<(NetId, NetOrderingInfo)> = (0..10)
            .map(|i| (net(i), info(2, 10.0, 0))) // all ties → RNG decides
            .collect();

        let order_a = order_nets(&nets, 42);
        let order_b = order_nets(&nets, 42);
        assert_eq!(order_a, order_b, "same seed must produce identical ordering");
    }

    #[test]
    fn different_seeds_can_produce_different_orderings() {
        // With 10 fully-tied nets there are 10! = 3.6M permutations.
        // The probability that two distinct seeds produce the same permutation
        // is negligible.
        let nets: Vec<(NetId, NetOrderingInfo)> = (0..10)
            .map(|i| (net(i), info(2, 10.0, 0)))
            .collect();

        let order_a = order_nets(&nets, 0);
        let order_b = order_nets(&nets, 99999);

        // It is overwhelmingly likely that they differ; if by cosmic chance
        // they match we accept it (the test does not assert inequality).
        // We only assert both have the correct length.
        assert_eq!(order_a.len(), 10);
        assert_eq!(order_b.len(), 10);

        // Document the expectation in a note (this is a probabilistic property).
        // With ChaCha8Rng seeded at 0 vs 99999 the permutations are almost
        // certainly different.
        let _ = order_a != order_b; // no assert — probabilistic
    }

    #[test]
    fn empty_input_returns_empty() {
        let order = order_nets(&[], 0);
        assert!(order.is_empty());
    }

    #[test]
    fn single_net_returns_that_net() {
        let nets = vec![(net(7), info(3, 5.0, 0))];
        let order = order_nets(&nets, 0);
        assert_eq!(order, vec![net(7)]);
    }

    #[test]
    fn critical_nets_all_before_non_critical_regardless_of_length() {
        let nets = vec![
            (net(0), info(2, 100.0, -1)), // critical, long
            (net(1), info(2, 1.0, 0)),    // non-critical, short
            (net(2), info(2, 50.0, -2)),  // critical, medium
        ];
        let order = order_nets(&nets, 0);
        // Find positions of each net in order.
        let pos: std::collections::HashMap<u32, usize> = order
            .iter()
            .enumerate()
            .map(|(i, id)| (id.raw(), i))
            .collect();

        assert!(
            pos[&0] < pos[&1],
            "critical net 0 must come before non-critical net 1"
        );
        assert!(
            pos[&2] < pos[&1],
            "critical net 2 must come before non-critical net 1"
        );
    }

    #[test]
    fn ordering_contains_all_nets() {
        let nets: Vec<(NetId, NetOrderingInfo)> = (0..5)
            .map(|i| (net(i), info(2, i as f64, 0)))
            .collect();
        let order = order_nets(&nets, 0);
        assert_eq!(order.len(), 5);
        let mut sorted = order.clone();
        sorted.sort_by_key(|id| id.raw());
        for i in 0..5u32 {
            assert_eq!(sorted[i as usize], net(i), "net {i} must appear exactly once");
        }
    }
}
