//! Design rules extracted from PcbDoc.

use crate::handles::{LayerId, RuleId};
use altium_format_types::pcb::{CornerStyle, NetTopology, RuleKind};

/// Scope selector for one side of a rule scope pair.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum IrRuleScope {
    All,
    NetClass(String),
    Layer(LayerId),
    NetClassAndLayer(String, LayerId),
}

/// Two-sided scope for a design rule (matches Altium's scope1/scope2 model).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct IrRuleScopePair {
    pub scope1: IrRuleScope,
    pub scope2: IrRuleScope,
}

impl Default for IrRuleScopePair {
    fn default() -> Self {
        Self {
            scope1: IrRuleScope::All,
            scope2: IrRuleScope::All,
        }
    }
}

/// A design rule from the PcbDoc.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct IrDesignRule {
    pub id: RuleId,
    pub name: String,
    pub kind: RuleKind,
    pub priority: i32,
    pub enabled: bool,
    pub scope: IrRuleScopePair,
    pub params: IrRuleParams,
}

/// Typed rule parameters (mm values).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum IrRuleParams {
    Clearance {
        gap_mm: f64,
    },
    Width {
        min_mm: f64,
        max_mm: f64,
        preferred_mm: f64,
    },
    ComponentClearance {
        gap_mm: f64,
    },
    BoardOutlineClearance {
        gap_mm: f64,
    },
    HoleToHoleClearance {
        gap_mm: f64,
    },
    MinimumAnnularRing {
        min_mm: f64,
    },
    SolderMaskExpansion {
        expansion_mm: f64,
    },
    PasteMaskExpansion {
        expansion_mm: f64,
    },
    RoutingTopology {
        topology: NetTopology,
    },
    RoutingPriority {
        priority: i32,
    },
    /// Layers allowed for routing. Only layers whose name matches a copper
    /// layer in the IR layer stack are included (unknown names are skipped).
    RoutingLayers {
        allowed: Vec<LayerId>,
    },
    RoutingViaStyle {
        width_min_mm: f64,
        width_max_mm: f64,
        hole_min_mm: f64,
        hole_max_mm: f64,
    },
    RoutingCornerStyle {
        style: CornerStyle,
    },
    DiffPairsRouting {
        gap_mm: f64,
        max_gap_mm: f64,
        max_uncoupled_length_mm: f64,
    },
    MatchedLengths {
        tolerance_mm: f64,
    },
    ShortCircuit,
    BrokenNets,
    NetAntennae,
    ViasUnderSmd,
    AcuteAngle {
        min_angle_deg: f64,
    },
    SmdToCorner {
        clearance_mm: f64,
    },
    MaximumViaCount {
        max: u32,
    },
    MaxMinHoleSize {
        min_mm: f64,
        max_mm: f64,
    },
    Length {
        min_mm: f64,
        max_mm: f64,
    },
    DaisyChainStubLength {
        max_mm: f64,
    },
    SmdNeckDown,
    SmdEntry,
    ParallelSegment {
        max_run_mm: f64,
        check_gap_mm: f64,
    },
    MinimumSolderMaskSliver {
        min_mm: f64,
    },
    SilkToSolderMaskClearance {
        clearance_mm: f64,
    },
    SilkToSilkClearance {
        clearance_mm: f64,
    },
    SilkToBoardRegionClearance {
        clearance_mm: f64,
    },
    PowerPlaneClearance {
        gap_mm: f64,
    },
    /// Thermal relief pattern validation for polygon connections.
    PolygonConnectStyle,
    Creepage {
        min_mm: f64,
    },
    MaxMinHeight {
        min_mm: f64,
        max_mm: f64,
    },
    ZAxisClearance {
        min_mm: f64,
    },
    /// Rule kind not yet given a typed IR representation.
    Other {
        kind: RuleKind,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handles::RuleId;

    #[test]
    fn design_rule_default_scope_is_all() {
        let rule = IrDesignRule {
            id: RuleId::from(0u32),
            name: "test".into(),
            kind: RuleKind::Clearance,
            priority: 1,
            enabled: true,
            scope: IrRuleScopePair::default(),
            params: IrRuleParams::Clearance { gap_mm: 0.25 },
        };
        assert_eq!(rule.scope.scope1, IrRuleScope::All);
        assert_eq!(rule.scope.scope2, IrRuleScope::All);
    }

    #[test]
    fn design_rule_net_class_scope() {
        let rule = IrDesignRule {
            id: RuleId::from(1u32),
            name: "high_speed".into(),
            kind: RuleKind::Clearance,
            priority: 2,
            enabled: true,
            scope: IrRuleScopePair {
                scope1: IrRuleScope::NetClass("HighSpeed".into()),
                scope2: IrRuleScope::All,
            },
            params: IrRuleParams::Clearance { gap_mm: 0.15 },
        };
        assert_eq!(
            rule.scope.scope1,
            IrRuleScope::NetClass("HighSpeed".into())
        );
        assert_eq!(rule.scope.scope2, IrRuleScope::All);
    }
}
