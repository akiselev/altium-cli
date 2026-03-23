//! Per-cell present-usage tracking for PathFinder negotiation.
//!
//! Unlike [`super::history::HistoryArray`] which accumulates across iterations,
//! `PresentUsageArray` is rebuilt from scratch each iteration to reflect the
//! current routing state. This implements OrthoRoute's "Refresh" pattern:
//! separate "where is congestion right now" from "where has congestion been
//! historically".
//!
//! Linearization matches `HistoryArray`:
//! `x * (grid_height * layer_count) + y * layer_count + layer`

// ---------------------------------------------------------------------------
// PresentUsageArray
// ---------------------------------------------------------------------------

/// Per-cell current-iteration net occupancy counts.
///
/// Each cell tracks how many nets currently route through it. Cells with
/// `usage >= 2` are oversubscribed and will be penalised by the cost function:
/// `pres_fac * max(0, usage - 1)`.
#[derive(Debug, Clone)]
pub struct PresentUsageArray {
    data: Vec<u16>,
    width: u32,
    height: u32,
    layer_count: usize,
}

impl PresentUsageArray {
    /// Create a new array for a grid of `width × height` cells and
    /// `layer_count` layers, initialised to all zeros.
    pub fn new(width: u32, height: u32, layer_count: usize) -> Self {
        let total = width as usize * height as usize * layer_count;
        PresentUsageArray {
            data: vec![0; total],
            width,
            height,
            layer_count,
        }
    }

    /// Reset all cells to zero. Called at the start of each iteration before
    /// rebuilding usage from `solution_paths`.
    pub fn clear(&mut self) {
        self.data.fill(0);
    }

    /// Linearize `(x, y, layer)` to a flat index.
    #[inline]
    fn index(&self, x: u32, y: u32, layer: u16) -> Option<usize> {
        if x >= self.width || y >= self.height || layer as usize >= self.layer_count {
            return None;
        }
        let h = self.height as usize;
        let l = self.layer_count;
        Some(x as usize * (h * l) + y as usize * l + layer as usize)
    }

    /// Increment the occupancy count at `(x, y, layer)` by 1.
    ///
    /// Out-of-bounds coordinates are silently ignored.
    pub fn increment(&mut self, x: u32, y: u32, layer: u16) {
        if let Some(idx) = self.index(x, y, layer) {
            self.data[idx] = self.data[idx].saturating_add(1);
        }
    }

    /// Return the occupancy count at `(x, y, layer)`.
    ///
    /// Returns `0` for out-of-bounds coordinates.
    pub fn get(&self, x: u32, y: u32, layer: u16) -> u16 {
        match self.index(x, y, layer) {
            Some(idx) => self.data[idx],
            None => 0,
        }
    }

    /// Return a slice over the entire flat array for passing to the
    /// detailed router's `present_usage` parameter.
    pub fn as_slice(&self) -> &[u16] {
        &self.data
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
        let u = PresentUsageArray::new(4, 4, 2);
        for x in 0..4u32 {
            for y in 0..4u32 {
                for layer in 0..2u16 {
                    assert_eq!(u.get(x, y, layer), 0);
                }
            }
        }
    }

    #[test]
    fn increment_and_get() {
        let mut u = PresentUsageArray::new(5, 5, 2);
        u.increment(2, 3, 0);
        u.increment(2, 3, 0);
        assert_eq!(u.get(2, 3, 0), 2);
        assert_eq!(u.get(2, 3, 1), 0);
        assert_eq!(u.get(0, 0, 0), 0);
    }

    #[test]
    fn clear_resets_to_zero() {
        let mut u = PresentUsageArray::new(3, 3, 1);
        u.increment(1, 1, 0);
        u.increment(2, 2, 0);
        u.clear();
        assert_eq!(u.get(1, 1, 0), 0);
        assert_eq!(u.get(2, 2, 0), 0);
    }

    #[test]
    fn out_of_bounds_returns_zero() {
        let u = PresentUsageArray::new(3, 3, 2);
        assert_eq!(u.get(3, 0, 0), 0);
        assert_eq!(u.get(0, 3, 0), 0);
        assert_eq!(u.get(0, 0, 2), 0);
    }

    #[test]
    fn out_of_bounds_increment_is_noop() {
        let mut u = PresentUsageArray::new(2, 2, 1);
        u.increment(5, 5, 0); // should not panic
        assert_eq!(u.get(0, 0, 0), 0);
    }

    #[test]
    fn as_slice_length_matches_dimensions() {
        let u = PresentUsageArray::new(5, 3, 4);
        assert_eq!(u.as_slice().len(), 5 * 3 * 4);
    }

    #[test]
    fn saturating_add_does_not_overflow() {
        let mut u = PresentUsageArray::new(1, 1, 1);
        for _ in 0..70_000 {
            u.increment(0, 0, 0);
        }
        assert_eq!(u.get(0, 0, 0), u16::MAX);
    }
}
