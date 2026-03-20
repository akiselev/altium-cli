//! `RouteSolutionBuilder` — accumulates per-net paths and iteration snapshots.
//!
//! Builds the final `autopcb_routes::RouteSolution` after PathFinder
//! negotiation completes.

use std::collections::BTreeMap;

use autopcb_routes::{
    NetId, RouteSolution, RoutedNet, RoutedVia, RoutingIterationSnapshot, RoutingMetrics,
    TraceSegment,
};

// ---------------------------------------------------------------------------
// RouteSolutionBuilder
// ---------------------------------------------------------------------------

/// Accumulates routing results from the PathFinder negotiation loop and
/// produces a complete [`RouteSolution`].
#[derive(Debug, Default)]
pub struct RouteSolutionBuilder {
    nets: BTreeMap<NetId, (Vec<TraceSegment>, Vec<RoutedVia>)>,
    unrouted: Vec<NetId>,
    snapshots: Vec<RoutingIterationSnapshot>,
    drc_violation_count: u32,
}

impl RouteSolutionBuilder {
    /// Create a new empty builder.
    pub fn new() -> Self {
        RouteSolutionBuilder::default()
    }

    /// Record a successfully routed net with its segments and vias.
    pub fn add_net(
        &mut self,
        net_id: NetId,
        segments: Vec<TraceSegment>,
        vias: Vec<RoutedVia>,
    ) {
        self.nets.insert(net_id, (segments, vias));
    }

    /// Record a net that could not be routed.
    pub fn add_unrouted(&mut self, net_id: NetId) {
        self.unrouted.push(net_id);
    }

    /// Append an iteration snapshot for viewer playback.
    pub fn add_snapshot(&mut self, snapshot: RoutingIterationSnapshot) {
        self.snapshots.push(snapshot);
    }

    /// Record the DRC violation count from the final full DRC run.
    pub fn set_drc_violations(&mut self, count: u32) {
        self.drc_violation_count = count;
    }

    /// Consume the builder and produce a [`RouteSolution`] with computed metrics.
    pub fn build(self) -> RouteSolution {
        let mut routed_nets: BTreeMap<NetId, RoutedNet> = BTreeMap::new();
        let mut total_length_mm = 0.0f64;
        let mut total_vias = 0u32;

        for (net_id, (segments, vias)) in self.nets {
            let routed_length_mm: f64 = segments.iter().map(|s| {
                let dx = s.end.x - s.start.x;
                let dy = s.end.y - s.start.y;
                (dx * dx + dy * dy).sqrt()
            }).sum();

            total_length_mm += routed_length_mm;
            total_vias += vias.len() as u32;

            routed_nets.insert(
                net_id,
                RoutedNet {
                    net_id,
                    segments,
                    vias,
                    routed_length_mm,
                },
            );
        }

        let routed_count = routed_nets.len() as u32;
        let unrouted_count = self.unrouted.len() as u32;
        let total_count = routed_count + unrouted_count;
        let completion_pct = if total_count == 0 {
            100.0
        } else {
            routed_count as f64 / total_count as f64 * 100.0
        };

        let metrics = RoutingMetrics {
            total_length_mm,
            total_vias,
            unrouted_count,
            completion_pct,
            drc_violations: self.drc_violation_count,
        };

        RouteSolution {
            version: autopcb_routes::CURRENT_VERSION,
            nets: routed_nets,
            unrouted: self.unrouted,
            metrics,
            iterations: self.snapshots,
            drc_violation_records: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use autopcb_routes::{LayerId, NetId, Point, TraceSegment};

    fn make_segment(net_id: NetId, x0: f64, y0: f64, x1: f64, y1: f64) -> TraceSegment {
        TraceSegment {
            net_id,
            layer: LayerId(0),
            start: Point { x: x0, y: y0 },
            end: Point { x: x1, y: y1 },
            width_mm: 0.2,
        }
    }

    #[test]
    fn empty_builder_produces_empty_solution() {
        let builder = RouteSolutionBuilder::new();
        let solution = builder.build();
        assert!(solution.nets.is_empty());
        assert!(solution.unrouted.is_empty());
        assert_eq!(solution.metrics.total_length_mm, 0.0);
        assert_eq!(solution.metrics.total_vias, 0);
        assert_eq!(solution.metrics.unrouted_count, 0);
        assert!((solution.metrics.completion_pct - 100.0).abs() < f64::EPSILON);
        assert_eq!(solution.metrics.drc_violations, 0);
        assert!(solution.iterations.is_empty());
    }

    #[test]
    fn add_net_routed_correctly() {
        let mut builder = RouteSolutionBuilder::new();
        let net_id = NetId(1);
        // 3mm horizontal segment.
        let seg = make_segment(net_id, 0.0, 0.0, 3.0, 0.0);
        builder.add_net(net_id, vec![seg], vec![]);
        let solution = builder.build();

        assert!(solution.nets.contains_key(&net_id));
        let net = &solution.nets[&net_id];
        assert_eq!(net.segments.len(), 1);
        assert!((net.routed_length_mm - 3.0).abs() < 1e-9);
        assert_eq!(solution.metrics.unrouted_count, 0);
        assert!((solution.metrics.completion_pct - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn add_unrouted_reported_in_metrics() {
        let mut builder = RouteSolutionBuilder::new();
        builder.add_net(NetId(0), vec![make_segment(NetId(0), 0.0, 0.0, 1.0, 0.0)], vec![]);
        builder.add_unrouted(NetId(1));
        let solution = builder.build();

        assert_eq!(solution.unrouted, vec![NetId(1)]);
        assert_eq!(solution.metrics.unrouted_count, 1);
        // completion = 1 routed / (1 routed + 1 unrouted) * 100 = 50%
        assert!((solution.metrics.completion_pct - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn snapshots_preserved_in_order() {
        let mut builder = RouteSolutionBuilder::new();
        for i in 0..3u32 {
            builder.add_snapshot(RoutingIterationSnapshot {
                iteration: i,
                conflicts: i * 2,
                routed_count: 5,
                unrouted_count: 0,
                paths: BTreeMap::new(),
            });
        }
        let solution = builder.build();
        assert_eq!(solution.iterations.len(), 3);
        for (i, snap) in solution.iterations.iter().enumerate() {
            assert_eq!(snap.iteration, i as u32);
        }
    }

    #[test]
    fn total_length_sums_all_nets() {
        let mut builder = RouteSolutionBuilder::new();
        // Net 0: 3mm segment.
        builder.add_net(NetId(0), vec![make_segment(NetId(0), 0.0, 0.0, 3.0, 0.0)], vec![]);
        // Net 1: 4mm segment.
        builder.add_net(NetId(1), vec![make_segment(NetId(1), 0.0, 0.0, 4.0, 0.0)], vec![]);
        let solution = builder.build();
        assert!((solution.metrics.total_length_mm - 7.0).abs() < 1e-9);
    }

    #[test]
    fn via_count_sums_all_nets() {
        let mut builder = RouteSolutionBuilder::new();
        let via = RoutedVia {
            net_id: NetId(0),
            position: Point { x: 0.0, y: 0.0 },
            from_layer: LayerId(0),
            to_layer: LayerId(1),
            drill_mm: 0.3,
            annular_ring_mm: 0.1,
        };
        builder.add_net(NetId(0), vec![], vec![via.clone(), via.clone()]);
        builder.add_net(NetId(1), vec![], vec![via]);
        let solution = builder.build();
        assert_eq!(solution.metrics.total_vias, 3);
    }

    #[test]
    fn completion_pct_all_unrouted() {
        let mut builder = RouteSolutionBuilder::new();
        builder.add_unrouted(NetId(0));
        builder.add_unrouted(NetId(1));
        let solution = builder.build();
        assert!((solution.metrics.completion_pct - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn drc_violations_is_zero() {
        let builder = RouteSolutionBuilder::new();
        let solution = builder.build();
        assert_eq!(solution.metrics.drc_violations, 0);
    }
}
