//! DRC policy: design rule resolution for validation.
//!
//! Mirrors `RoutingPolicy` but covers ALL DRC-checkable rules (not just
//! routing-time rules). Uses `BTreeMap` for deterministic resolution order.

use std::collections::BTreeMap;

use autopcb_ir::{IrDesignRule, IrRuleParams, PcbIr};
use autopcb_routes::LayerId;

use super::DrcError;

/// Width bounds for DRC checking (min/max/preferred in mm).
#[derive(Debug, Clone, Copy)]
pub struct DrcWidthBounds {
    pub min_mm: f64,
    pub max_mm: f64,
    pub preferred_mm: f64,
}

/// Via bounds for DRC checking (all values in mm).
#[derive(Debug, Clone, Copy)]
pub struct DrcViaBounds {
    pub hole_min_mm: f64,
    pub hole_max_mm: f64,
    pub annular_ring_min_mm: f64,
    pub max_via_count: Option<u32>,
    pub hole_to_hole_clearance_mm: f64,
}

impl Default for DrcViaBounds {
    fn default() -> Self {
        Self {
            hole_min_mm: 0.1,
            hole_max_mm: 6.35,
            annular_ring_min_mm: 0.05,
            max_via_count: None,
            hole_to_hole_clearance_mm: 0.25,
        }
    }
}

/// Length constraints for DRC.
#[derive(Debug, Clone, Copy)]
pub struct LengthConstraint {
    pub min_mm: f64,
    pub max_mm: f64,
}

/// Diff-pair constraints for DRC.
#[derive(Debug, Clone, Copy)]
pub struct DiffPairConstraint {
    pub gap_mm: f64,
    pub max_gap_mm: f64,
    pub max_uncoupled_length_mm: f64,
}

/// Matched-length constraint (tolerance in mm).
#[derive(Debug, Clone, Copy)]
pub struct MatchedLengthConstraint {
    pub tolerance_mm: f64,
}

/// Clearance matrix: per-net-class-pair clearance values.
///
/// Stored as a flat `Vec<f64>` indexed by `(class_a, class_b)` where
/// class indices come from `class_map`. Symmetric: `matrix[a][b] == matrix[b][a]`.
#[derive(Debug, Clone)]
pub struct ClearanceMatrix {
    /// Map from net class name to index. Empty string key = default class.
    class_map: BTreeMap<String, usize>,
    /// Flat matrix: `values[a * size + b]` gives clearance between classes a and b.
    values: Vec<f64>,
    /// Number of classes (matrix dimension).
    size: usize,
    /// Default clearance (used when no specific rule exists).
    default_clearance_mm: f64,
}

impl ClearanceMatrix {
    /// Look up the clearance between two net classes by name.
    pub fn clearance(&self, class_a: &str, class_b: &str) -> f64 {
        let idx_a = self.class_map.get(class_a).copied();
        let idx_b = self.class_map.get(class_b).copied();
        match (idx_a, idx_b) {
            (Some(a), Some(b)) => self.values[a * self.size + b],
            _ => self.default_clearance_mm,
        }
    }

    /// Look up clearance between two net classes, falling back to default.
    pub fn clearance_or_default(&self, class_a: Option<&str>, class_b: Option<&str>) -> f64 {
        let a = class_a.unwrap_or("");
        let b = class_b.unwrap_or("");
        self.clearance(a, b)
    }
}

/// DRC policy built from `PcbIr` design rules.
///
/// All lookups use `BTreeMap` for deterministic iteration order.
#[derive(Debug, Clone)]
pub struct DrcPolicy {
    pub clearance_matrix: ClearanceMatrix,
    pub width_constraints: BTreeMap<Option<String>, DrcWidthBounds>,
    pub via_bounds: DrcViaBounds,
    pub board_outline_clearance_mm: f64,
    pub component_clearance_mm: f64,
    pub matched_length: Option<MatchedLengthConstraint>,
    pub diff_pair: Option<DiffPairConstraint>,
    /// Per-net-class length bounds.  Key `None` = default (all nets).
    pub length_constraints: BTreeMap<Option<String>, LengthConstraint>,
    pub solder_mask_expansion_mm: f64,
    pub paste_mask_expansion_mm: f64,
    pub acute_angle_min_deg: f64,
    /// Minimum clearance from a trace corner (bend) to an SMD pad edge, in mm.
    pub smd_to_corner_clearance_mm: f64,
}

impl DrcPolicy {
    /// Build DrcPolicy from PcbIr design rules.
    pub fn build(ir: &PcbIr) -> Result<Self, DrcError> {
        // Collect enabled rules, sorted by priority (lower number = higher priority).
        let mut sorted_rules: Vec<&IrDesignRule> = ir
            .rules
            .values()
            .filter(|r| r.enabled)
            .collect();
        sorted_rules.sort_by_key(|r| r.priority);

        // Extract default clearance.
        let default_clearance_mm = sorted_rules
            .iter()
            .find_map(|r| match &r.params {
                IrRuleParams::Clearance { gap_mm } => Some(*gap_mm),
                _ => None,
            })
            .unwrap_or(0.1);

        // Build clearance matrix.
        // Currently IR doesn't carry net-class scope, so we have a single default class.
        let mut class_map = BTreeMap::new();
        class_map.insert(String::new(), 0); // default class at index 0
        let clearance_matrix = ClearanceMatrix {
            class_map,
            values: vec![default_clearance_mm],
            size: 1,
            default_clearance_mm,
        };

        // Width constraints: build from all Width rules.
        let mut width_constraints = BTreeMap::new();
        for r in &sorted_rules {
            if let IrRuleParams::Width { min_mm, max_mm, preferred_mm } = &r.params {
                // Currently no per-net-class scoping, so None key = default.
                width_constraints.entry(None).or_insert(DrcWidthBounds {
                    min_mm: *min_mm,
                    max_mm: *max_mm,
                    preferred_mm: *preferred_mm,
                });
            }
        }
        // Ensure a default exists.
        width_constraints.entry(None).or_insert(DrcWidthBounds {
            min_mm: 0.1,
            max_mm: 3.0,
            preferred_mm: 0.2,
        });

        // Via bounds.
        let mut via_bounds = DrcViaBounds::default();
        for r in &sorted_rules {
            match &r.params {
                IrRuleParams::RoutingViaStyle { hole_min_mm, hole_max_mm, .. } => {
                    via_bounds.hole_min_mm = *hole_min_mm;
                    via_bounds.hole_max_mm = *hole_max_mm;
                    break;
                }
                _ => {}
            }
        }
        for r in &sorted_rules {
            match &r.params {
                IrRuleParams::MinimumAnnularRing { min_mm } => {
                    via_bounds.annular_ring_min_mm = *min_mm;
                    break;
                }
                _ => {}
            }
        }
        for r in &sorted_rules {
            match &r.params {
                IrRuleParams::HoleToHoleClearance { gap_mm } => {
                    via_bounds.hole_to_hole_clearance_mm = *gap_mm;
                    break;
                }
                _ => {}
            }
        }

        // Board outline clearance.
        let board_outline_clearance_mm = sorted_rules
            .iter()
            .find_map(|r| match &r.params {
                IrRuleParams::BoardOutlineClearance { gap_mm } => Some(*gap_mm),
                _ => None,
            })
            .unwrap_or(0.5);

        // Component clearance.
        let component_clearance_mm = sorted_rules
            .iter()
            .find_map(|r| match &r.params {
                IrRuleParams::ComponentClearance { gap_mm } => Some(*gap_mm),
                _ => None,
            })
            .unwrap_or(0.25);

        // Matched length.
        let matched_length = sorted_rules.iter().find_map(|r| match &r.params {
            IrRuleParams::MatchedLengths { tolerance_mm } => {
                Some(MatchedLengthConstraint { tolerance_mm: *tolerance_mm })
            }
            _ => None,
        });

        // Diff pair.
        let diff_pair = sorted_rules.iter().find_map(|r| match &r.params {
            IrRuleParams::DiffPairsRouting { gap_mm, max_gap_mm, max_uncoupled_length_mm } => {
                Some(DiffPairConstraint {
                    gap_mm: *gap_mm,
                    max_gap_mm: *max_gap_mm,
                    max_uncoupled_length_mm: *max_uncoupled_length_mm,
                })
            }
            _ => None,
        });

        // Length constraints (min/max per net).
        let mut length_constraints: BTreeMap<Option<String>, LengthConstraint> = BTreeMap::new();
        for r in &sorted_rules {
            if let IrRuleParams::Length { min_mm, max_mm } = &r.params {
                length_constraints.entry(None).or_insert(LengthConstraint {
                    min_mm: *min_mm,
                    max_mm: *max_mm,
                });
                break;
            }
        }

        // Solder/paste mask expansion.
        let solder_mask_expansion_mm = sorted_rules
            .iter()
            .find_map(|r| match &r.params {
                IrRuleParams::SolderMaskExpansion { expansion_mm } => Some(*expansion_mm),
                _ => None,
            })
            .unwrap_or(0.1);

        let paste_mask_expansion_mm = sorted_rules
            .iter()
            .find_map(|r| match &r.params {
                IrRuleParams::PasteMaskExpansion { expansion_mm } => Some(*expansion_mm),
                _ => None,
            })
            .unwrap_or(0.0);

        // Acute angle threshold.
        let acute_angle_min_deg = sorted_rules
            .iter()
            .find_map(|r| match &r.params {
                IrRuleParams::AcuteAngle { min_angle_deg } => Some(*min_angle_deg),
                _ => None,
            })
            .unwrap_or(45.0);

        // SMD-to-corner clearance.
        let smd_to_corner_clearance_mm = sorted_rules
            .iter()
            .find_map(|r| match &r.params {
                IrRuleParams::SmdToCorner { clearance_mm } => Some(*clearance_mm),
                _ => None,
            })
            .unwrap_or(0.0);

        Ok(DrcPolicy {
            clearance_matrix,
            width_constraints,
            via_bounds,
            board_outline_clearance_mm,
            component_clearance_mm,
            matched_length,
            diff_pair,
            length_constraints,
            solder_mask_expansion_mm,
            paste_mask_expansion_mm,
            acute_angle_min_deg,
            smd_to_corner_clearance_mm,
        })
    }

    /// Get clearance between two net classes.
    pub fn clearance(&self, class_a: Option<&str>, class_b: Option<&str>) -> f64 {
        self.clearance_matrix.clearance_or_default(class_a, class_b)
    }

    /// Get width bounds for a net class on a given layer.
    pub fn width_bounds(&self, net_class: Option<&str>, _layer: Option<LayerId>) -> DrcWidthBounds {
        let _ = _layer;
        // Look up by net class name, fall back to default (None key).
        if let Some(class) = net_class {
            if let Some(bounds) = self.width_constraints.get(&Some(class.to_string())) {
                return *bounds;
            }
        }
        *self.width_constraints.get(&None).unwrap()
    }

    /// Get via bounds for a net class.
    pub fn via_bounds(&self) -> &DrcViaBounds {
        &self.via_bounds
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use autopcb_ir::{
        handles::{IdMap, LayerId as IrLayerId, NetId as IrNetId, RuleId},
        layer_stack::{IrCopperLayer, IrLayerStack, PreferredDirection},
        rule::{IrDesignRule, IrRuleParams},
        types::{BoundingBoxMm, PointMm},
        IrBoardGeometry, PcbIr,
    };
    use altium_format_types::pcb::RuleKind;

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
                copper_layers: vec![
                    IrCopperLayer {
                        id: IrLayerId::from(0u32),
                        name: "Top Layer".into(),
                        is_top: true,
                        is_bottom: false,
                        preferred_direction: Some(PreferredDirection::Any),
                    },
                    IrCopperLayer {
                        id: IrLayerId::from(1u32),
                        name: "Bottom Layer".into(),
                        is_top: false,
                        is_bottom: true,
                        preferred_direction: Some(PreferredDirection::Any),
                    },
                ],
                copper_layer_count: 2,
            },
            components: IdMap::new(),
            nets: IdMap::new(),
            rules: IdMap::new(),
            free_copper: Default::default(),
            polygons: IdMap::new(),
        }
    }

    fn add_rule(ir: &mut PcbIr, kind: RuleKind, priority: i32, params: IrRuleParams) {
        let id = ir.rules.push(IrDesignRule {
            id: RuleId::from(0u32),
            name: "test_rule".into(),
            kind,
            priority,
            enabled: true,
            params,
        });
        ir.rules[id].id = id;
    }

    #[test]
    fn single_clearance_rule_all_pairs() {
        let mut ir = empty_ir();
        add_rule(&mut ir, RuleKind::Clearance, 1, IrRuleParams::Clearance { gap_mm: 0.25 });
        let policy = DrcPolicy::build(&ir).unwrap();
        let gap = policy.clearance(None, None);
        assert!((gap - 0.25).abs() < f64::EPSILON, "expected 0.25 mm, got {gap}");
    }

    #[test]
    fn default_clearance_when_no_rule() {
        let ir = empty_ir();
        let policy = DrcPolicy::build(&ir).unwrap();
        let gap = policy.clearance(None, None);
        assert!((gap - 0.1).abs() < f64::EPSILON, "expected 0.1 mm default, got {gap}");
    }

    #[test]
    fn conflicting_width_rules_higher_priority_wins() {
        let mut ir = empty_ir();
        // priority=2: wider, lower priority
        add_rule(&mut ir, RuleKind::Width, 2, IrRuleParams::Width {
            min_mm: 0.5, max_mm: 5.0, preferred_mm: 1.0,
        });
        // priority=1: narrower, higher priority (lower number)
        add_rule(&mut ir, RuleKind::Width, 1, IrRuleParams::Width {
            min_mm: 0.1, max_mm: 1.0, preferred_mm: 0.2,
        });
        let policy = DrcPolicy::build(&ir).unwrap();
        let bounds = policy.width_bounds(None, None);
        assert!((bounds.preferred_mm - 0.2).abs() < f64::EPSILON,
            "expected preferred=0.2 (high-priority rule), got {}", bounds.preferred_mm);
    }

    #[test]
    fn width_bounds_default_when_no_rule() {
        let ir = empty_ir();
        let policy = DrcPolicy::build(&ir).unwrap();
        let bounds = policy.width_bounds(None, None);
        assert!((bounds.min_mm - 0.1).abs() < f64::EPSILON);
        assert!((bounds.max_mm - 3.0).abs() < f64::EPSILON);
        assert!((bounds.preferred_mm - 0.2).abs() < f64::EPSILON);
    }

    #[test]
    #[ignore] // Enable when IrDesignRule carries net-class scope
    fn net_class_specific_clearance_overrides_default() {
        // TODO: validates that a Power-class clearance rule overrides the
        // default for Power-to-Signal pairs once IR supports net-class scoping.
    }
}
