//! Design rules extracted from PcbDoc.

use crate::handles::RuleId;
use altium_format_types::pcb::RuleKind;

/// A design rule from the PcbDoc.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct IrDesignRule {
    pub id: RuleId,
    pub name: String,
    pub kind: RuleKind,
    pub priority: i32,
    pub enabled: bool,
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
    /// Rule kind not yet given a typed IR representation.
    Other {
        kind: RuleKind,
    },
}
