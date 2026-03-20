//! GPU-accelerated DRC engine.

use autopcb_ir::PcbIr;
use autopcb_routes::RouteSolution;

use crate::drc::{DrcEngine, DrcError};
use crate::drc::cpu_engine::CpuDrcEngine;
use crate::drc::policy::DrcPolicy;
use crate::drc::report::DrcReport;
use crate::workspace::RoutingWorkspace;

use super::GpuContext;

/// GPU-accelerated DRC engine.
///
/// Currently delegates to CPU engine. GPU compute shaders for parallel
/// sweepline clearance checking will be added in a future iteration when
/// the segment count exceeds the GPU threshold.
pub struct GpuDrcEngine {
    gpu_ctx: GpuContext,
    cpu_fallback: CpuDrcEngine,
    /// Segment count threshold above which GPU is used.
    gpu_threshold: usize,
}

impl GpuDrcEngine {
    pub fn new(gpu_ctx: GpuContext, policy: DrcPolicy, gpu_threshold: usize) -> Self {
        let cpu_fallback = CpuDrcEngine::new(policy);
        Self {
            gpu_ctx,
            cpu_fallback,
            gpu_threshold,
        }
    }

    /// Count total segments in solution.
    fn segment_count(solution: &RouteSolution) -> usize {
        solution.nets.values().map(|n| n.segments.len()).sum()
    }
}

impl DrcEngine for GpuDrcEngine {
    fn check_routing(
        &self,
        solution: &RouteSolution,
        workspace: &RoutingWorkspace,
        ir: &PcbIr,
    ) -> Result<DrcReport, DrcError> {
        // GPU acceleration for routing-time DRC not yet implemented.
        // Fall back to CPU.
        self.cpu_fallback.check_routing(solution, workspace, ir)
    }

    fn check_full(
        &self,
        solution: &RouteSolution,
        workspace: &RoutingWorkspace,
        ir: &PcbIr,
    ) -> Result<DrcReport, DrcError> {
        let count = Self::segment_count(solution);
        if count < self.gpu_threshold {
            tracing::debug!(
                segments = count,
                threshold = self.gpu_threshold,
                "segment count below GPU threshold, using CPU DRC"
            );
            return self.cpu_fallback.check_full(solution, workspace, ir);
        }

        // TODO: GPU parallel sweepline clearance check
        // For now, fall back to CPU even above threshold.
        tracing::debug!(
            segments = count,
            "GPU DRC not yet implemented, falling back to CPU"
        );
        self.cpu_fallback.check_full(solution, workspace, ir)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "gpu-tests"))]
mod tests {
    use super::*;
    use autopcb_ir::{
        handles::{IdMap, LayerId as IrLayerId},
        layer_stack::{IrCopperLayer, IrLayerStack, PreferredDirection},
        types::{BoundingBoxMm, PointMm},
        IrBoardGeometry, PcbIr,
    };
    use autopcb_routes::{LayerId, NetId, Point, RoutedNet, RouteSolution, TraceSegment};
    use crate::config::RoutingConfig;
    use crate::workspace::build_workspace;

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

    fn make_workspace(ir: &PcbIr) -> RoutingWorkspace {
        let mut config = RoutingConfig::default();
        config.grid_resolution_mm = 1.0;
        build_workspace(ir, &config).unwrap()
    }

    #[test]
    fn gpu_context_try_new_is_graceful() {
        // GpuContext::try_new() must return Some or None without panicking.
        // On headless CI with no GPU adapter, None is the expected result.
        let _ctx = GpuContext::try_new();
        // No assertion — any non-panic outcome is correct.
    }

    #[test]
    fn gpu_drc_falls_back_to_cpu_below_threshold() {
        let Some(ctx) = GpuContext::try_new() else {
            return; // No GPU available — graceful skip.
        };
        let ir = empty_ir();
        let policy = DrcPolicy::build(&ir).unwrap();
        // Threshold of 1000: empty solution has 0 segments, always below.
        let engine = GpuDrcEngine::new(ctx, policy, 1000);
        let solution = RouteSolution::new();
        let workspace = make_workspace(&ir);

        let report = engine.check_routing(&solution, &workspace, &ir).unwrap();
        assert!(report.is_clean(), "empty solution should have no violations");
    }

    #[test]
    fn gpu_drc_matches_cpu_drc() {
        // NOTE: GPU shaders are stubs. This test verifies that the GPU engine's
        // CPU fallback produces identical results to the standalone CPU engine.
        // It does NOT validate GPU compute correctness.
        let Some(ctx) = GpuContext::try_new() else {
            return; // No GPU available — graceful skip.
        };
        let ir = empty_ir();
        let policy_gpu = DrcPolicy::build(&ir).unwrap();
        let policy_cpu = DrcPolicy::build(&ir).unwrap();

        let gpu_engine = GpuDrcEngine::new(ctx, policy_gpu, 0);
        let cpu_engine = CpuDrcEngine::new(policy_cpu);

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

        let workspace = make_workspace(&ir);

        let gpu_report = gpu_engine.check_full(&solution, &workspace, &ir).unwrap();
        let cpu_report = cpu_engine.check_full(&solution, &workspace, &ir).unwrap();

        assert_eq!(
            gpu_report.total_count(),
            cpu_report.total_count(),
            "GPU and CPU DRC must produce identical violation counts"
        );
    }
}
