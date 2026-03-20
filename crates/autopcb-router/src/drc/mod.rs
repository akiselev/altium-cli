//! Design-rule checking for route solutions.

pub mod board;
pub mod repair;
pub mod clearance;
pub mod connectivity;
pub mod cpu_engine;
pub mod diff_pair;
pub mod geometry;
pub mod length;
pub mod manufacturing;
pub mod policy;
pub mod report;
pub mod shorts;
pub mod topology;
pub mod via;
pub mod width;

use std::fmt;

use autopcb_ir::types::PointMm;
use autopcb_routes::{LayerId, NetId, RoutedVia, RouteSolution, TraceSegment};
use altium_format_types::pcb::RuleKind;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Shared net-level geometry helpers
// ---------------------------------------------------------------------------

/// Compute the Euclidean length of all segments in the solution for one net.
pub(crate) fn net_length_mm(solution: &RouteSolution, net_id: NetId) -> f64 {
    solution
        .nets
        .get(&net_id)
        .map(|rn| {
            rn.segments
                .iter()
                .map(|s| {
                    let dx = s.end.x - s.start.x;
                    let dy = s.end.y - s.start.y;
                    (dx * dx + dy * dy).sqrt()
                })
                .sum()
        })
        .unwrap_or(0.0)
}

/// Midpoint of the first segment of a routed net, used as violation location.
pub(crate) fn net_midpoint(solution: &RouteSolution, net_id: NetId) -> PointMm {
    solution
        .nets
        .get(&net_id)
        .and_then(|rn| rn.segments.first())
        .map(|s| PointMm {
            x: (s.start.x + s.end.x) / 2.0,
            y: (s.start.y + s.end.y) / 2.0,
        })
        .unwrap_or(PointMm { x: 0.0, y: 0.0 })
}

use crate::RoutingError;

/// Errors specific to DRC operations.
#[derive(Debug, Error)]
pub enum DrcError {
    #[error("unsupported rule kind for DRC: {kind}")]
    UnsupportedRule { kind: String },

    #[error("DRC policy build error: {0}")]
    PolicyBuildError(String),

    #[error("DRC check failed: {0}")]
    CheckFailed(String),
}

impl From<DrcError> for RoutingError {
    fn from(e: DrcError) -> Self {
        RoutingError::RoutingFailed(format!("DRC error: {e}"))
    }
}

/// A physical object involved in a DRC violation.
#[derive(Debug, Clone)]
pub enum DrcObject {
    Segment(TraceSegment),
    Via(RoutedVia),
    Pad {
        component: String,
        pad: String,
        position: PointMm,
    },
    Keepout {
        id: usize,
    },
    BoardEdge,
    Component {
        designator: String,
    },
    Polygon {
        id: usize,
    },
}

/// Classification of a DRC violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DrcViolationKind {
    ClearanceViolation,
    ShortCircuit,
    WidthBelowMinimum,
    WidthAboveMaximum,
    AnnularRingBelowMinimum,
    HoleSizeBelowMinimum,
    HoleSizeAboveMaximum,
    HoleToHoleClearance,
    ViaCountExceeded,
    AcuteAngle,
    SmdToCorner,
    BrokenNet,
    NetAntenna,
    LengthBelowMinimum,
    LengthAboveMaximum,
    MatchedLengthExceeded,
    DiffPairGapViolation,
    DiffPairWidthMismatch,
    DiffPairUncoupledLength,
    DiffPairSkew,
    DaisyChainStubLength,
    BoardOutlineClearance,
    ComponentClearance,
    SolderMaskSliver,
    SilkToSolderMask,
    SilkToSilk,
    SilkToBoardRegion,
    ViasUnderSmd,
    ParallelSegment,
}

impl fmt::Display for DrcViolationKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

/// A single DRC violation with full context.
#[derive(Debug, Clone)]
pub struct DrcViolation {
    pub kind: DrcViolationKind,
    pub rule_kind: RuleKind,
    pub rule_name: String,
    pub object_a: DrcObject,
    pub object_b: Option<DrcObject>,
    pub location: PointMm,
    pub layer: Option<LayerId>,
    pub actual_mm: f64,
    pub required_mm: f64,
}

/// DRC engine trait — implemented by CPU and GPU backends.
pub trait DrcEngine {
    /// Fast routing-time check: clearance + shorts only.
    /// Returns violations for PathFinder to penalize.
    /// Does NOT take history parameter — history updates happen in PathFinder loop.
    fn check_routing(
        &self,
        solution: &autopcb_routes::RouteSolution,
        workspace: &crate::workspace::RoutingWorkspace,
        ir: &autopcb_ir::PcbIr,
    ) -> Result<report::DrcReport, DrcError>;

    /// Comprehensive post-route check: all applicable rules.
    fn check_full(
        &self,
        solution: &autopcb_routes::RouteSolution,
        workspace: &crate::workspace::RoutingWorkspace,
        ir: &autopcb_ir::PcbIr,
    ) -> Result<report::DrcReport, DrcError>;
}

/// Identifies an individual DRC check category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrcCheckKind {
    Clearance,
    Shorts,
    Width,
    Via,
    Geometry,
    Connectivity,
    Length,
    DiffPair,
    Board,
    Manufacturing,
    Topology,
}

/// DRC configuration for PathFinder integration.
#[derive(Debug, Clone)]
pub struct DrcConfig {
    /// First PathFinder iteration to run DRC (skip early noisy iterations).
    pub start_iteration: u32,
    /// Penalty added to history costs for each DRC violation.
    pub violation_penalty: f64,
    /// Whether DRC is enabled at all.
    pub enabled: bool,
    /// Which individual check categories are enabled.
    pub enabled_checks: Vec<DrcCheckKind>,
}

impl Default for DrcConfig {
    fn default() -> Self {
        Self {
            start_iteration: 3,
            violation_penalty: 10.0,
            enabled: true,
            enabled_checks: vec![
                DrcCheckKind::Clearance,
                DrcCheckKind::Shorts,
                DrcCheckKind::Width,
                DrcCheckKind::Via,
                DrcCheckKind::Geometry,
                DrcCheckKind::Connectivity,
                DrcCheckKind::Length,
                DrcCheckKind::DiffPair,
                DrcCheckKind::Board,
                DrcCheckKind::Manufacturing,
                DrcCheckKind::Topology,
            ],
        }
    }
}
