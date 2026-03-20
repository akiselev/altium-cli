//! CPU-based DRC engine implementing the `DrcEngine` trait.

use autopcb_ir::PcbIr;
use autopcb_routes::RouteSolution;

use crate::workspace::RoutingWorkspace;

use super::{DrcEngine, DrcError};
use super::policy::DrcPolicy;
use super::report::DrcReport;
use super::clearance::check_clearance;
use super::shorts::check_shorts;
use super::width::check_widths;
use super::via::check_vias;
use super::geometry::check_geometry;
use super::connectivity::check_connectivity;
use super::length::check_lengths;
use super::diff_pair::check_diff_pairs;
use super::board::check_board;
use super::manufacturing::check_manufacturing;
use super::topology::check_topology;
use super::{board, geometry, via};

/// CPU-based DRC engine.
pub struct CpuDrcEngine {
    policy: DrcPolicy,
}

impl CpuDrcEngine {
    pub fn new(policy: DrcPolicy) -> Self {
        Self { policy }
    }
}

impl DrcEngine for CpuDrcEngine {
    fn check_routing(
        &self,
        solution: &RouteSolution,
        _workspace: &RoutingWorkspace,
        ir: &PcbIr,
    ) -> Result<DrcReport, DrcError> {
        let mut violations = Vec::new();
        violations.extend(check_clearance(solution, &self.policy, ir));
        violations.extend(check_shorts(solution));
        Ok(DrcReport::new(violations))
    }

    fn check_full(
        &self,
        solution: &RouteSolution,
        _workspace: &RoutingWorkspace,
        ir: &PcbIr,
    ) -> Result<DrcReport, DrcError> {
        let mut violations = Vec::new();
        violations.extend(check_clearance(solution, &self.policy, ir));
        violations.extend(check_shorts(solution));
        violations.extend(check_widths(solution, &self.policy));
        violations.extend(check_vias(solution, &self.policy));
        violations.extend(check_geometry(solution, &self.policy, ir));
        violations.extend(check_connectivity(solution, ir, &self.policy));
        violations.extend(check_lengths(solution, &self.policy));
        violations.extend(check_diff_pairs(solution, &self.policy, ir));
        violations.extend(check_board(solution, ir, &self.policy));
        violations.extend(check_manufacturing(solution, ir, &self.policy));
        violations.extend(check_topology(solution, &self.policy));
        violations.extend(geometry::check_parallel_segments(solution, &self.policy));
        violations.extend(via::check_vias_under_smd(solution, ir));
        violations.extend(board::check_component_clearance(solution, ir, &self.policy));
        violations.extend(board::check_creepage(solution, ir, &self.policy));
        // 15 checker dispatches above (clearance, shorts, widths, vias, geometry,
        // connectivity, lengths, diff_pairs, board, manufacturing, topology,
        // parallel_segments, vias_under_smd, component_clearance, creepage).
        let report = DrcReport::new(violations)
            .with_audit(
                15,
                self.policy.skipped_rules.clone(),
            );
        Ok(report)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use autopcb_ir::{
        handles::{IdMap, LayerId as IrLayerId},
        layer_stack::{IrCopperLayer, IrLayerStack, PreferredDirection},
        types::{BoundingBoxMm, PointMm},
        IrBoardGeometry, PcbIr,
    };
    use autopcb_routes::{LayerId, NetId, Point, RoutedNet, RouteSolution, TraceSegment};
    use crate::workspace::build_workspace;
    use crate::config::RoutingConfig;

    fn empty_ir() -> PcbIr {
        PcbIr {
            board: IrBoardGeometry {
                outline: vec![],
                cutouts: vec![],
                bounds: BoundingBoxMm {
                    min: PointMm { x: 0.0, y: 0.0 },
                    max: PointMm { x: 100.0, y: 100.0 },
                },
                keepouts: vec![],
            },
            layer_stack: IrLayerStack {
                copper_layers: vec![IrCopperLayer {
                    id: IrLayerId::from(0u32),
                    name: "Top".into(),
                    is_top: true,
                    is_bottom: false,
                    preferred_direction: Some(PreferredDirection::Any),
                }],
                copper_layer_count: 1,
            },
            components: IdMap::new(),
            nets: IdMap::new(),
            rules: IdMap::new(),
            free_copper: Default::default(),
            polygons: IdMap::new(),
            texts: IdMap::new(),
            regions: IdMap::new(),
            component_bodies: IdMap::new(),
        }
    }

    fn make_workspace(ir: &PcbIr) -> crate::workspace::RoutingWorkspace {
        let mut config = RoutingConfig::default();
        config.grid_resolution_mm = 1.0;
        build_workspace(ir, &config).unwrap()
    }

    #[test]
    fn check_routing_empty_solution_no_violations() {
        let ir = empty_ir();
        let policy = DrcPolicy::build(&ir).unwrap();
        let engine = CpuDrcEngine::new(policy);
        let solution = RouteSolution::new();
        let workspace = make_workspace(&ir);

        let report = engine.check_routing(&solution, &workspace, &ir).unwrap();
        assert!(report.is_clean(), "empty solution should have no violations");
        assert_eq!(report.total_count(), 0);
    }

    #[test]
    fn check_full_empty_solution_no_violations() {
        let ir = empty_ir();
        let policy = DrcPolicy::build(&ir).unwrap();
        let engine = CpuDrcEngine::new(policy);
        let solution = RouteSolution::new();
        let workspace = make_workspace(&ir);

        let report = engine.check_full(&solution, &workspace, &ir).unwrap();
        assert!(report.is_clean(), "empty solution should have no violations in full check");
    }

    #[test]
    fn check_routing_detects_short_circuit() {
        let ir = empty_ir();
        let policy = DrcPolicy::build(&ir).unwrap();
        let engine = CpuDrcEngine::new(policy);
        let workspace = make_workspace(&ir);

        // Two crossing segments from different nets on the same layer.
        let layer = LayerId(0);
        let seg_a = TraceSegment {
            net_id: NetId(0),
            layer,
            start: Point { x: -1.0, y: 0.0 },
            end: Point { x: 1.0, y: 0.0 },
            width_mm: 0.2,
        };
        let seg_b = TraceSegment {
            net_id: NetId(1),
            layer,
            start: Point { x: 0.0, y: -1.0 },
            end: Point { x: 0.0, y: 1.0 },
            width_mm: 0.2,
        };

        let mut solution = RouteSolution::new();
        solution.nets.insert(NetId(0), RoutedNet {
            net_id: NetId(0),
            segments: vec![seg_a],
            vias: vec![],
            routed_length_mm: 2.0,
        });
        solution.nets.insert(NetId(1), RoutedNet {
            net_id: NetId(1),
            segments: vec![seg_b],
            vias: vec![],
            routed_length_mm: 2.0,
        });

        let report = engine.check_routing(&solution, &workspace, &ir).unwrap();
        assert!(!report.is_clean(), "crossing segments should produce a short violation");
        let shorts: Vec<_> = report.violations
            .iter()
            .filter(|v| v.kind == crate::drc::DrcViolationKind::ShortCircuit)
            .collect();
        assert!(!shorts.is_empty(), "expected at least one ShortCircuit violation");
    }

    #[test]
    fn check_full_detects_violations() {
        let ir = empty_ir();
        let policy = DrcPolicy::build(&ir).unwrap();
        let engine = CpuDrcEngine::new(policy);
        let workspace = make_workspace(&ir);

        // Two parallel traces that violate default clearance (0.1 mm).
        let layer = LayerId(0);
        let seg_a = TraceSegment {
            net_id: NetId(0),
            layer,
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 1.0, y: 0.0 },
            width_mm: 0.1,
        };
        // 0.05mm between edges: actual = 0.1 - 0.05 - 0.05 = 0.0, required = 0.1 → violation.
        let seg_b = TraceSegment {
            net_id: NetId(1),
            layer,
            start: Point { x: 0.0, y: 0.1 },
            end: Point { x: 1.0, y: 0.1 },
            width_mm: 0.1,
        };

        let mut solution = RouteSolution::new();
        solution.nets.insert(NetId(0), RoutedNet {
            net_id: NetId(0),
            segments: vec![seg_a],
            vias: vec![],
            routed_length_mm: 1.0,
        });
        solution.nets.insert(NetId(1), RoutedNet {
            net_id: NetId(1),
            segments: vec![seg_b],
            vias: vec![],
            routed_length_mm: 1.0,
        });

        let report = engine.check_full(&solution, &workspace, &ir).unwrap();
        assert!(!report.is_clean(), "traces violating clearance should produce violations");
        let counts = report.count_by_rule();
        assert!(counts.len() > 0, "should have at least one rule category with violations");
    }
}
