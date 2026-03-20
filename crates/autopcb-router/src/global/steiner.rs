//! Net decomposition: multi-pin nets → 2-pin subnets.
//!
//! `MstDecomposer` uses `petgraph::algo::min_spanning_tree` on a complete
//! graph with Euclidean distance weights to decompose an n-pin net into
//! n−1 two-pin subnets.  The `NetDecomposer` trait interface allows a FLUTE
//! backend to be substituted for near-optimal Steiner tree decomposition on
//! high-fanout nets without changing call sites.
//!
//! # MST properties
//!
//! For a complete graph on n pins:
//! - n − 1 edges in the MST (Steiner tree lower bound is ≥ MST total length)
//! - Edge selection is deterministic: petgraph breaks ties by node index order,
//!   which is fixed by the caller-provided pin slice ordering.

use autopcb_ir::types::PointMm;
use autopcb_routes::NetId;
use petgraph::algo::min_spanning_tree;
use petgraph::data::Element;
use petgraph::graph::{NodeIndex, UnGraph};

// ---------------------------------------------------------------------------
// CellId newtype
// ---------------------------------------------------------------------------

/// Identifies a cell in the coarse global routing grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellId(pub u32);

// ---------------------------------------------------------------------------
// Subnet
// ---------------------------------------------------------------------------

/// A 2-pin routing subnet produced by net decomposition.
#[derive(Debug, Clone)]
pub struct Subnet {
    /// Source pin position in mm.
    pub source: PointMm,
    /// Target pin position in mm.
    pub target: PointMm,
    /// Net this subnet belongs to.
    pub net_id: NetId,
    /// Preferred start layer for the detailed router. `None` means use the
    /// first layer allowed by policy. Set by layer assignment (M5) or the
    /// layer assignment API consumer.
    pub source_layer: Option<autopcb_routes::LayerId>,
    /// Preferred goal layer for the detailed router. `None` means use the
    /// first layer allowed by policy.
    pub target_layer: Option<autopcb_routes::LayerId>,
    /// Coarse routing path through global grid cells (populated after global
    /// routing; empty until `global_route` fills it in).
    pub region_path: Vec<CellId>,
}

// ---------------------------------------------------------------------------
// NetDecomposer trait
// ---------------------------------------------------------------------------

/// Decomposes a multi-pin net into a set of 2-pin subnets.
///
/// The canonical implementation is `MstDecomposer`; FLUTE-based backends can
/// implement this trait to provide near-optimal Steiner tree decomposition for
/// high-fanout nets.
pub trait NetDecomposer {
    /// Decompose `pins` into 2-pin subnets for the given `net_id`.
    ///
    /// - 0 or 1 pins → returns an empty `Vec`.
    /// - n ≥ 2 pins  → returns exactly n − 1 subnets.
    fn decompose(&self, pins: &[PointMm], net_id: NetId) -> Vec<Subnet>;
}

// ---------------------------------------------------------------------------
// MstDecomposer
// ---------------------------------------------------------------------------

/// MST-based net decomposer using `petgraph::algo::min_spanning_tree`.
///
/// Builds a complete graph over the pin set with edge weights equal to
/// Euclidean distance, then extracts the MST edges as 2-pin subnets.
///
/// # Complexity
///
/// O(n² log n) where n = number of pins.  Acceptable for typical PCB nets
/// (n ≤ 50).  For very high-fanout nets (n > 200) substitute a FLUTE backend.
#[derive(Debug, Default, Clone, Copy)]
pub struct MstDecomposer;

impl NetDecomposer for MstDecomposer {
    fn decompose(&self, pins: &[PointMm], net_id: NetId) -> Vec<Subnet> {
        let n = pins.len();
        if n < 2 {
            return Vec::new();
        }

        // Build a complete undirected graph.  petgraph requires integer or
        // ordered-float edge weights; we store the raw f64 bits.  The MST
        // algorithm uses `PartialOrd` which is correct for non-NaN distances.
        let mut g: UnGraph<usize, f64> = UnGraph::new_undirected();

        let nodes: Vec<NodeIndex> = (0..n).map(|i| g.add_node(i)).collect();

        for i in 0..n {
            for j in (i + 1)..n {
                let dist = pins[i].distance_to(&pins[j]);
                g.add_edge(nodes[i], nodes[j], dist);
            }
        }

        // Run Prim's MST.  `min_spanning_tree` returns an iterator of
        // `Element::Edge { source, target, weight }` and `Element::Node`.
        let mst_elements: Vec<Element<usize, f64>> = min_spanning_tree(&g).collect();

        let mut subnets = Vec::with_capacity(n - 1);
        for elem in mst_elements {
            if let Element::Edge {
                source,
                target,
                weight: _,
            } = elem
            {
                subnets.push(Subnet {
                    source: pins[source],
                    target: pins[target],
                    net_id,
                    source_layer: None,
                    target_layer: None,
                    region_path: Vec::new(),
                });
            }
        }

        subnets
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: f64, y: f64) -> PointMm {
        PointMm::new(x, y)
    }

    fn net(id: u32) -> NetId {
        NetId(id)
    }

    #[test]
    fn one_pin_produces_no_subnets() {
        let d = MstDecomposer;
        let result = d.decompose(&[pt(0.0, 0.0)], net(0));
        assert!(result.is_empty(), "1-pin net should produce 0 subnets");
    }

    #[test]
    fn zero_pins_produces_no_subnets() {
        let d = MstDecomposer;
        let result = d.decompose(&[], net(0));
        assert!(result.is_empty(), "0-pin net should produce 0 subnets");
    }

    #[test]
    fn two_pin_net_produces_one_subnet() {
        let d = MstDecomposer;
        let pins = [pt(0.0, 0.0), pt(10.0, 0.0)];
        let result = d.decompose(&pins, net(1));
        assert_eq!(result.len(), 1, "2-pin net should produce 1 subnet");
        assert_eq!(result[0].net_id, net(1));
    }

    #[test]
    fn three_pin_l_shape_produces_two_subnets() {
        let d = MstDecomposer;
        // L-shape: (0,0), (5,0), (5,5)
        let pins = [pt(0.0, 0.0), pt(5.0, 0.0), pt(5.0, 5.0)];
        let result = d.decompose(&pins, net(2));
        assert_eq!(result.len(), 2, "3-pin net should produce 2 subnets");
    }

    #[test]
    fn four_pin_square_produces_three_subnets() {
        let d = MstDecomposer;
        // Square corners at (0,0), (10,0), (10,10), (0,10)
        let pins = [pt(0.0, 0.0), pt(10.0, 0.0), pt(10.0, 10.0), pt(0.0, 10.0)];
        let result = d.decompose(&pins, net(3));
        assert_eq!(result.len(), 3, "4-pin net should produce 3 subnets");

        // Total MST length must be ≤ full perimeter (40mm).
        // MST of a square = 3 sides = 30mm.
        let total: f64 = result
            .iter()
            .map(|s| s.source.distance_to(&s.target))
            .sum();
        assert!(
            total <= 40.0 + 1e-9,
            "MST total length {total} should be ≤ 40mm (perimeter)"
        );
    }

    #[test]
    fn subnet_net_id_matches_input() {
        let d = MstDecomposer;
        let pins = [pt(0.0, 0.0), pt(1.0, 0.0), pt(2.0, 0.0)];
        let result = d.decompose(&pins, net(42));
        for s in &result {
            assert_eq!(s.net_id, net(42), "all subnets should carry the input net_id");
        }
    }

    #[test]
    fn subnet_region_path_starts_empty() {
        let d = MstDecomposer;
        let pins = [pt(0.0, 0.0), pt(5.0, 0.0)];
        let result = d.decompose(&pins, net(0));
        assert_eq!(result.len(), 1);
        assert!(
            result[0].region_path.is_empty(),
            "region_path should be empty before global routing"
        );
    }

    // Property: MST edge count = n-1 for n-pin net (n >= 2)
    #[test]
    fn mst_edge_count_is_n_minus_one() {
        let d = MstDecomposer;
        for n in 2usize..=10 {
            let pins: Vec<PointMm> = (0..n).map(|i| pt(i as f64, 0.0)).collect();
            let result = d.decompose(&pins, net(0));
            assert_eq!(
                result.len(),
                n - 1,
                "MST of {n}-pin net should have {n}-1 subnets, got {}",
                result.len()
            );
        }
    }
}
