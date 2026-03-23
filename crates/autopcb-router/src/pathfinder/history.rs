//! PathFinder history congestion array.
//!
//! History cells are linearized as
//! `x * (grid_height * layer_count) + y * layer_count + layer.raw() as usize`.
//! Grid dimensions are fixed at workspace build time.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// HistoryArray
// ---------------------------------------------------------------------------

/// Per-cell history congestion costs for PathFinder negotiation.
///
/// Cells that are oversubscribed accumulate history cost each iteration,
/// steering future routes away from persistently contested resources.
#[derive(Debug, Clone)]
pub struct HistoryArray {
    data: Vec<f64>,
    width: u32,
    height: u32,
    layer_count: usize,
}

impl HistoryArray {
    /// Create a new history array for a grid of `width × height` cells and
    /// `layer_count` layers, initialised to all zeros.
    pub fn new(width: u32, height: u32, layer_count: usize) -> Self {
        let total = width as usize * height as usize * layer_count;
        HistoryArray {
            data: vec![0.0; total],
            width,
            height,
            layer_count,
        }
    }

    /// Linearize `(x, y, layer)` to a flat index.
    ///
    /// Formula: `x * (height * layer_count) + y * layer_count + layer`
    #[inline]
    fn index(&self, x: u32, y: u32, layer: u16) -> Option<usize> {
        if x >= self.width || y >= self.height || layer as usize >= self.layer_count {
            return None;
        }
        let h = self.height as usize;
        let l = self.layer_count;
        Some(x as usize * (h * l) + y as usize * l + layer as usize)
    }

    /// Return the history cost at `(x, y, layer)`.
    ///
    /// Returns `0.0` for out-of-bounds coordinates.
    pub fn get(&self, x: u32, y: u32, layer: u16) -> f64 {
        match self.index(x, y, layer) {
            Some(idx) => self.data[idx],
            None => 0.0,
        }
    }

    /// Add `amount` to the history cost at `(x, y, layer)`.
    ///
    /// Out-of-bounds coordinates are silently ignored.
    pub fn increment(&mut self, x: u32, y: u32, layer: u16, amount: f64) {
        if let Some(idx) = self.index(x, y, layer) {
            self.data[idx] += amount;
        }
    }

    /// Return a slice over the entire flat history array for passing to the
    /// detailed router's `history_costs` parameter.
    pub fn as_slice(&self) -> &[f64] {
        &self.data
    }

    /// Multiply all history values by `factor`.
    ///
    /// Use with `factor < 1.0` (e.g. 0.95) to gradually "forget" old
    /// congestion history, preventing route fossilization. A factor of 1.0
    /// is a no-op.
    pub fn decay(&mut self, factor: f64) {
        if (factor - 1.0).abs() < f64::EPSILON {
            return; // no-op for factor == 1.0
        }
        for v in &mut self.data {
            *v *= factor;
        }
    }
}

// ---------------------------------------------------------------------------
// EdgeHistoryMap
// ---------------------------------------------------------------------------

/// Per-edge history congestion costs.
///
/// Unlike the dense cell-based `HistoryArray`, edge history uses a sparse
/// HashMap since only a small fraction of edges are typically oversubscribed.
///
/// An edge key is `((x1, y1, layer1), (x2, y2, layer2))` in canonical
/// (lexicographically sorted) order so that A→B and B→A map to the same entry.
#[derive(Debug, Clone)]
pub struct EdgeHistoryMap {
    data: HashMap<((u32, u32, u16), (u32, u32, u16)), f64>,
}

impl EdgeHistoryMap {
    /// Create a new, empty edge history map.
    pub fn new() -> Self {
        EdgeHistoryMap { data: HashMap::new() }
    }

    /// Return the history cost for `edge`, or `0.0` if the edge has no entry.
    pub fn get(&self, edge: &((u32, u32, u16), (u32, u32, u16))) -> f64 {
        self.data.get(edge).copied().unwrap_or(0.0)
    }

    /// Add `amount` to the history cost for `edge`.
    pub fn increment(&mut self, edge: ((u32, u32, u16), (u32, u32, u16)), amount: f64) {
        *self.data.entry(edge).or_insert(0.0) += amount;
    }

    /// Multiply all stored history values by `factor`.
    ///
    /// Use with `factor < 1.0` to gradually forget old congestion history.
    /// A factor of exactly `1.0` is a no-op (entries are left untouched).
    pub fn decay(&mut self, factor: f64) {
        if (factor - 1.0).abs() < f64::EPSILON {
            return;
        }
        for v in self.data.values_mut() {
            *v *= factor;
        }
    }
}

impl Default for EdgeHistoryMap {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_array_all_zeros() {
        let h = HistoryArray::new(4, 4, 2);
        for x in 0..4u32 {
            for y in 0..4u32 {
                for layer in 0..2u16 {
                    assert_eq!(
                        h.get(x, y, layer),
                        0.0,
                        "cell ({x},{y},layer {layer}) should be 0.0 initially"
                    );
                }
            }
        }
    }

    #[test]
    fn increment_and_get_roundtrip() {
        let mut h = HistoryArray::new(5, 5, 2);
        h.increment(2, 3, 0, 1.5);
        assert!(
            (h.get(2, 3, 0) - 1.5).abs() < f64::EPSILON,
            "get should return the incremented value"
        );
        // Other cells unaffected.
        assert_eq!(h.get(2, 3, 1), 0.0);
        assert_eq!(h.get(0, 0, 0), 0.0);
    }

    #[test]
    fn increment_accumulates() {
        let mut h = HistoryArray::new(4, 4, 2);
        h.increment(1, 1, 0, 2.0);
        h.increment(1, 1, 0, 3.0);
        assert!(
            (h.get(1, 1, 0) - 5.0).abs() < f64::EPSILON,
            "accumulated value should be 5.0"
        );
    }

    #[test]
    fn linearization_is_consistent() {
        let mut h = HistoryArray::new(3, 4, 2);
        // Set distinct values at every cell, then read them back.
        let mut expected = Vec::new();
        let mut val = 1.0f64;
        for x in 0..3u32 {
            for y in 0..4u32 {
                for layer in 0..2u16 {
                    h.increment(x, y, layer, val);
                    expected.push((x, y, layer, val));
                    val += 1.0;
                }
            }
        }
        for (x, y, layer, v) in expected {
            assert!(
                (h.get(x, y, layer) - v).abs() < f64::EPSILON,
                "cell ({x},{y},layer {layer}) should be {v}"
            );
        }
    }

    #[test]
    fn out_of_bounds_get_returns_zero() {
        let h = HistoryArray::new(3, 3, 2);
        assert_eq!(h.get(3, 0, 0), 0.0, "x out of bounds");
        assert_eq!(h.get(0, 3, 0), 0.0, "y out of bounds");
        assert_eq!(h.get(0, 0, 2), 0.0, "layer out of bounds");
    }

    #[test]
    fn out_of_bounds_increment_is_noop() {
        let mut h = HistoryArray::new(2, 2, 1);
        h.increment(5, 5, 0, 99.0); // should not panic
        assert_eq!(h.get(0, 0, 0), 0.0, "no cell should have changed");
    }

    #[test]
    fn as_slice_length_matches_dimensions() {
        let h = HistoryArray::new(5, 3, 4);
        assert_eq!(h.as_slice().len(), 5 * 3 * 4);
    }

    #[test]
    fn decay_multiplies_all_values() {
        let mut h = HistoryArray::new(3, 3, 1);
        h.increment(0, 0, 0, 10.0);
        h.increment(1, 1, 0, 20.0);
        h.decay(0.5);
        assert!((h.get(0, 0, 0) - 5.0).abs() < f64::EPSILON);
        assert!((h.get(1, 1, 0) - 10.0).abs() < f64::EPSILON);
        assert_eq!(h.get(2, 2, 0), 0.0, "zero stays zero after decay");
    }

    #[test]
    fn decay_one_is_noop() {
        let mut h = HistoryArray::new(2, 2, 1);
        h.increment(0, 0, 0, 5.0);
        h.decay(1.0);
        assert!((h.get(0, 0, 0) - 5.0).abs() < f64::EPSILON);
    }

    // --- EdgeHistoryMap tests ------------------------------------------------

    #[test]
    fn edge_history_map_increment_and_get() {
        let mut m = EdgeHistoryMap::new();
        let edge = ((1u32, 2u32, 0u16), (1u32, 3u32, 0u16));
        assert_eq!(m.get(&edge), 0.0, "unknown edge should return 0.0");
        m.increment(edge, 2.5);
        assert!(
            (m.get(&edge) - 2.5).abs() < f64::EPSILON,
            "get should return the incremented value"
        );
        m.increment(edge, 1.0);
        assert!(
            (m.get(&edge) - 3.5).abs() < f64::EPSILON,
            "second increment should accumulate"
        );
    }

    #[test]
    fn edge_history_map_decay() {
        let mut m = EdgeHistoryMap::new();
        let edge = ((0u32, 0u32, 0u16), (1u32, 0u32, 0u16));
        m.increment(edge, 10.0);
        m.decay(0.5);
        assert!(
            (m.get(&edge) - 5.0).abs() < f64::EPSILON,
            "decay by 0.5 should halve the value"
        );
    }

    #[test]
    fn edge_history_map_decay_one_is_noop() {
        let mut m = EdgeHistoryMap::new();
        let edge = ((0u32, 0u32, 0u16), (1u32, 0u32, 0u16));
        m.increment(edge, 7.0);
        m.decay(1.0);
        assert!((m.get(&edge) - 7.0).abs() < f64::EPSILON);
    }
}
