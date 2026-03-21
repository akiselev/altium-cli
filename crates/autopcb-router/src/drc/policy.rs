//! DRC policy: design rule resolution for validation.
//!
//! Mirrors `RoutingPolicy` but covers ALL DRC-checkable rules (not just
//! routing-time rules). Uses `BTreeMap` for deterministic resolution order.

use std::collections::BTreeMap;

use altium_format_types::pcb::RuleKind;
use autopcb_ir::{IrDesignRule, IrRuleParams, IrRuleScope, IrRuleScopePair, PcbIr};
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

/// Scope priority level for width/via rule cascade.
///
/// Higher value = higher priority. Used internally by `width_bounds` and `via_bounds`
/// to select the most-specific rule that matches the query context.
///
/// Priority order (per Decision Log): NetClassAndLayer > NetClass > Layer > All.
fn scope_priority(scope: &IrRuleScope) -> u8 {
    match scope {
        IrRuleScope::All => 0,
        IrRuleScope::Layer(_) => 1,
        IrRuleScope::NetClass(_) => 2,
        IrRuleScope::NetClassAndLayer(_, _) => 3,
    }
}

/// Returns true if `scope` matches the given net class and layer context.
fn scope_matches(
    scope: &IrRuleScope,
    net_class: Option<&str>,
    layer: Option<autopcb_ir::LayerId>,
) -> bool {
    match scope {
        IrRuleScope::All => true,
        IrRuleScope::NetClass(cls) => net_class.map_or(false, |nc| nc == cls.as_str()),
        IrRuleScope::Layer(lid) => layer.map_or(false, |l| l == *lid),
        IrRuleScope::NetClassAndLayer(cls, lid) => {
            net_class.map_or(false, |nc| nc == cls.as_str())
                && layer.map_or(false, |l| l == *lid)
        }
    }
}

/// DRC policy built from `PcbIr` design rules.
///
/// All lookups use `BTreeMap` for deterministic iteration order.
#[derive(Debug, Clone)]
pub struct DrcPolicy {
    pub clearance_matrix: ClearanceMatrix,
    /// Scoped clearance rules: `(scope_pair, gap_mm)` in priority order (highest priority first).
    ///
    /// Each entry represents a Clearance rule with its two-sided scope. Lookup via
    /// `clearance_for_scopes()` checks both orderings (scope1↔class_a AND scope2↔class_b,
    /// or scope1↔class_b AND scope2↔class_a) and returns the first match.
    pub clearance_scoped: Vec<(IrRuleScopePair, f64)>,
    /// Width constraints stored as `(scope, bounds)` pairs sorted by priority descending.
    /// Lookup uses explicit cascade: NetClassAndLayer > NetClass > Layer > All.
    pub width_constraints: Vec<(IrRuleScope, DrcWidthBounds)>,
    /// Via bounds stored as `(scope, bounds)` pairs sorted by priority descending.
    /// Lookup uses explicit cascade: NetClass > All (layer is not meaningful for vias).
    pub via_bounds_scoped: Vec<(IrRuleScope, DrcViaBounds)>,
    /// Kept for direct mutation in tests (e.g. setting max_via_count).
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
    pub parallel_segment_gap_mm: f64,
    pub parallel_segment_max_length_mm: f64,
    pub creepage_distance_mm: f64,
    pub daisy_chain_stub_max_mm: f64,
    pub skipped_rules: Vec<RuleKind>,
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

        // Extract default clearance (first All-scoped Clearance rule, or 0.1 mm).
        let default_clearance_mm = sorted_rules
            .iter()
            .find_map(|r| match &r.params {
                IrRuleParams::Clearance { gap_mm }
                    if matches!(r.scope.scope1, IrRuleScope::All)
                        && matches!(r.scope.scope2, IrRuleScope::All) =>
                {
                    Some(*gap_mm)
                }
                _ => None,
            })
            .or_else(|| {
                sorted_rules.iter().find_map(|r| match &r.params {
                    IrRuleParams::Clearance { gap_mm } => Some(*gap_mm),
                    _ => None,
                })
            })
            .unwrap_or(0.1);

        // Collect all scoped clearance rules (in priority order, highest priority first).
        let clearance_scoped: Vec<(IrRuleScopePair, f64)> = sorted_rules
            .iter()
            .filter_map(|r| match &r.params {
                IrRuleParams::Clearance { gap_mm } => Some((r.scope.clone(), *gap_mm)),
                _ => None,
            })
            .collect();

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

        // Width constraints: collect all Width rules as (scope, bounds), preserving
        // declaration order (rules are already sorted by priority). Cascade lookup
        // (width_bounds) selects the highest-specificity matching scope at query time.
        let mut width_constraints: Vec<(IrRuleScope, DrcWidthBounds)> = Vec::new();
        for r in &sorted_rules {
            if let IrRuleParams::Width { min_mm, max_mm, preferred_mm } = &r.params {
                width_constraints.push((
                    r.scope.scope1.clone(),
                    DrcWidthBounds {
                        min_mm: *min_mm,
                        max_mm: *max_mm,
                        preferred_mm: *preferred_mm,
                    },
                ));
            }
        }
        // Ensure a default All-scope entry exists.
        let has_all_scope = width_constraints
            .iter()
            .any(|(s, _)| matches!(s, IrRuleScope::All));
        if !has_all_scope {
            width_constraints.push((
                IrRuleScope::All,
                DrcWidthBounds { min_mm: 0.1, max_mm: 3.0, preferred_mm: 0.2 },
            ));
        }

        // Via bounds: collect per-scope RoutingViaStyle rules for cascade lookup.
        // Global scalar fields (annular ring, hole-to-hole) are stored in via_bounds
        // for backward compatibility.
        let mut via_bounds_scoped: Vec<(IrRuleScope, DrcViaBounds)> = Vec::new();
        for r in &sorted_rules {
            if let IrRuleParams::RoutingViaStyle { hole_min_mm, hole_max_mm, .. } = &r.params {
                via_bounds_scoped.push((
                    r.scope.scope1.clone(),
                    DrcViaBounds {
                        hole_min_mm: *hole_min_mm,
                        hole_max_mm: *hole_max_mm,
                        ..DrcViaBounds::default()
                    },
                ));
            }
        }
        // Ensure a default All-scope via entry exists.
        let has_all_via = via_bounds_scoped
            .iter()
            .any(|(s, _)| matches!(s, IrRuleScope::All));
        if !has_all_via {
            via_bounds_scoped.push((IrRuleScope::All, DrcViaBounds::default()));
        }

        // Global via bounds for direct mutation (max_via_count, annular ring, h2h).
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

        // Parallel segment constraints.
        let (parallel_segment_gap_mm, parallel_segment_max_length_mm) = sorted_rules
            .iter()
            .find_map(|r| match &r.params {
                IrRuleParams::ParallelSegment { max_run_mm, check_gap_mm } => {
                    Some((*check_gap_mm, *max_run_mm))
                }
                _ => None,
            })
            .unwrap_or((0.0, f64::MAX));

        // Creepage distance.
        let creepage_distance_mm = sorted_rules
            .iter()
            .find_map(|r| match &r.params {
                IrRuleParams::Creepage { min_mm } => Some(*min_mm),
                _ => None,
            })
            .unwrap_or(0.0);

        // Daisy chain stub max length.
        let daisy_chain_stub_max_mm = sorted_rules
            .iter()
            .find_map(|r| match &r.params {
                IrRuleParams::DaisyChainStubLength { max_mm } => Some(*max_mm),
                _ => None,
            })
            .unwrap_or(f64::MAX);

        // Collect skipped rules (rules with Other params).
        let skipped_rules: Vec<RuleKind> = sorted_rules
            .iter()
            .filter_map(|r| match &r.params {
                IrRuleParams::Other { kind } => Some(*kind),
                _ => None,
            })
            .collect();

        Ok(DrcPolicy {
            clearance_matrix,
            clearance_scoped,
            width_constraints,
            via_bounds_scoped,
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
            parallel_segment_gap_mm,
            parallel_segment_max_length_mm,
            creepage_distance_mm,
            daisy_chain_stub_max_mm,
            skipped_rules,
        })
    }

    /// Get clearance between two net classes, using scoped rules with cascade fallback.
    ///
    /// Checks `clearance_scoped` rules bidirectionally (scope1↔class_a AND scope2↔class_b,
    /// or scope1↔class_b AND scope2↔class_a), returning the first match in priority order.
    /// Falls back to `clearance_matrix` (default clearance) when no scoped rule matches.
    pub fn clearance(&self, class_a: Option<&str>, class_b: Option<&str>) -> f64 {
        self.clearance_for_scopes(class_a, class_b)
    }

    /// Look up clearance for a pair of net classes using scoped Clearance rules.
    ///
    /// Rules are checked in priority order (as stored in `clearance_scoped`).
    /// A rule matches when scope1 matches `class_a` AND scope2 matches `class_b`,
    /// OR scope1 matches `class_b` AND scope2 matches `class_a`.
    /// Falls back to the default clearance when no scoped rule matches.
    pub fn clearance_for_scopes(
        &self,
        class_a: Option<&str>,
        class_b: Option<&str>,
    ) -> f64 {
        for (scope_pair, gap_mm) in &self.clearance_scoped {
            let fwd = scope_matches(&scope_pair.scope1, class_a, None)
                && scope_matches(&scope_pair.scope2, class_b, None);
            let rev = scope_matches(&scope_pair.scope1, class_b, None)
                && scope_matches(&scope_pair.scope2, class_a, None);
            if fwd || rev {
                return *gap_mm;
            }
        }
        self.clearance_matrix.clearance_or_default(class_a, class_b)
    }

    /// Get width bounds for a net class on a given layer.
    ///
    /// Cascade priority: NetClassAndLayer > NetClass > Layer > All.
    /// Uses explicit match arms per Decision Log (no Ord on IrRuleScope).
    pub fn width_bounds(
        &self,
        net_class: Option<&str>,
        layer: Option<LayerId>,
    ) -> DrcWidthBounds {
        // Convert routes::LayerId to ir::LayerId for scope matching.
        let ir_layer = layer.map(|l| autopcb_ir::LayerId::from(l.0 as u32));

        let mut best: Option<(u8, DrcWidthBounds)> = None;
        for (scope, bounds) in &self.width_constraints {
            if scope_matches(scope, net_class, ir_layer) {
                let prio = scope_priority(scope);
                match best {
                    None => best = Some((prio, *bounds)),
                    Some((best_prio, _)) if prio > best_prio => best = Some((prio, *bounds)),
                    _ => {}
                }
            }
        }
        best.map(|(_, b)| b).unwrap_or(DrcWidthBounds {
            min_mm: 0.1,
            max_mm: 3.0,
            preferred_mm: 0.2,
        })
    }

    /// Get via bounds for a net class, applying cascade scope resolution.
    ///
    /// Cascade priority: NetClass > All (layer is not meaningful for vias).
    /// Uses explicit match arms per Decision Log (no Ord on IrRuleScope).
    ///
    /// Returns an owned `DrcViaBounds` merging global values (annular ring,
    /// hole-to-hole clearance, max via count) with any net-class-scoped
    /// hole size override from `via_bounds_scoped`.
    pub fn via_bounds_for(&self, net_class: Option<&str>) -> DrcViaBounds {
        let mut best_prio: u8 = 0;
        let mut best_hole: Option<(f64, f64)> = None;
        for (scope, bounds) in &self.via_bounds_scoped {
            // Only NetClass and All are meaningful for via scope.
            let matches = match scope {
                IrRuleScope::All => true,
                IrRuleScope::NetClass(cls) => {
                    net_class.map_or(false, |nc| nc == cls.as_str())
                }
                IrRuleScope::Layer(_) | IrRuleScope::NetClassAndLayer(_, _) => false,
            };
            if matches {
                let prio = scope_priority(scope);
                if best_hole.is_none() || prio > best_prio {
                    best_prio = prio;
                    best_hole = Some((bounds.hole_min_mm, bounds.hole_max_mm));
                }
            }
        }
        // Start from the global via_bounds (annular ring, h2h, max_via_count are global)
        // and apply scoped hole-size override if found.
        let mut result = self.via_bounds;
        if let Some((hole_min, hole_max)) = best_hole {
            result.hole_min_mm = hole_min;
            result.hole_max_mm = hole_max;
        }
        result
    }

    /// Get global via bounds (annular ring, hole-to-hole clearance, max via count).
    ///
    /// Returns `&self.via_bounds` which is populated from global `RoutingViaStyle`,
    /// `MinimumAnnularRing`, and `HoleToHoleClearance` rules.
    ///
    /// For per-net-class hole size lookup, use `via_bounds_for()`.
    pub fn global_via_bounds(&self) -> &DrcViaBounds {
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
        handles::{LayerId as IrLayerId, RuleId},
        rule::{IrDesignRule, IrRuleParams, IrRuleScopePair},
        PcbIr,
    };
    use altium_format_types::pcb::RuleKind;

    use super::super::test_helpers::empty_ir;

    fn add_rule(ir: &mut PcbIr, kind: RuleKind, priority: i32, params: IrRuleParams) {
        let id = ir.rules.push(IrDesignRule {
            id: RuleId::from(0u32),
            name: "test_rule".into(),
            kind,
            priority,
            enabled: true,
            scope: IrRuleScopePair::default(),
            params,
        });
        ir.rules[id].id = id;
    }

    fn add_scoped_width_rule(
        ir: &mut PcbIr,
        priority: i32,
        scope: IrRuleScope,
        min_mm: f64,
        max_mm: f64,
        preferred_mm: f64,
    ) {
        let id = ir.rules.push(IrDesignRule {
            id: RuleId::from(0u32),
            name: "width_rule".into(),
            kind: RuleKind::Width,
            priority,
            enabled: true,
            scope: IrRuleScopePair {
                scope1: scope,
                scope2: IrRuleScope::All,
            },
            params: IrRuleParams::Width { min_mm, max_mm, preferred_mm },
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
    fn net_class_specific_width_overrides_default() {
        let mut ir = empty_ir();
        // All-scope default: min=0.1
        add_scoped_width_rule(&mut ir, 2, IrRuleScope::All, 0.1, 3.0, 0.2);
        // Power-class override: min=0.3 (higher priority number, but higher scope specificity wins)
        add_scoped_width_rule(&mut ir, 1, IrRuleScope::NetClass("Power".into()), 0.3, 3.0, 0.5);

        let policy = DrcPolicy::build(&ir).unwrap();

        // Power net class gets the override.
        let power_bounds = policy.width_bounds(Some("Power"), None);
        assert!(
            (power_bounds.min_mm - 0.3).abs() < f64::EPSILON,
            "Power class should get min=0.3, got {}",
            power_bounds.min_mm
        );

        // Signal net class falls back to default.
        let signal_bounds = policy.width_bounds(Some("Signal"), None);
        assert!(
            (signal_bounds.min_mm - 0.1).abs() < f64::EPSILON,
            "Signal class should fall back to All min=0.1, got {}",
            signal_bounds.min_mm
        );
    }

    #[test]
    fn layer_specific_width_overrides_default() {
        let mut ir = empty_ir();
        // All-scope default: preferred=0.2
        add_scoped_width_rule(&mut ir, 2, IrRuleScope::All, 0.1, 3.0, 0.2);
        // Layer 0 override: preferred=0.15
        add_scoped_width_rule(
            &mut ir,
            1,
            IrRuleScope::Layer(IrLayerId::from(0u32)),
            0.05,
            1.0,
            0.15,
        );

        let policy = DrcPolicy::build(&ir).unwrap();

        // Layer 0 query gets the layer-specific rule.
        let bounds_layer0 = policy.width_bounds(None, Some(LayerId(0)));
        assert!(
            (bounds_layer0.preferred_mm - 0.15).abs() < f64::EPSILON,
            "Layer 0 should get preferred=0.15, got {}",
            bounds_layer0.preferred_mm
        );

        // Layer 1 falls back to All.
        let bounds_layer1 = policy.width_bounds(None, Some(LayerId(1)));
        assert!(
            (bounds_layer1.preferred_mm - 0.2).abs() < f64::EPSILON,
            "Layer 1 should fall back to All preferred=0.2, got {}",
            bounds_layer1.preferred_mm
        );
    }

    #[test]
    fn most_specific_scope_wins_class_and_layer_over_class_over_all() {
        let mut ir = empty_ir();
        // All-scope default
        add_scoped_width_rule(&mut ir, 3, IrRuleScope::All, 0.1, 3.0, 0.2);
        // NetClass-only
        add_scoped_width_rule(
            &mut ir,
            2,
            IrRuleScope::NetClass("Power".into()),
            0.2,
            3.0,
            0.3,
        );
        // NetClassAndLayer — most specific
        add_scoped_width_rule(
            &mut ir,
            1,
            IrRuleScope::NetClassAndLayer("Power".into(), IrLayerId::from(0u32)),
            0.4,
            3.0,
            0.6,
        );

        let policy = DrcPolicy::build(&ir).unwrap();

        // Power on layer 0 → NetClassAndLayer wins (preferred=0.6).
        let bounds = policy.width_bounds(Some("Power"), Some(LayerId(0)));
        assert!(
            (bounds.preferred_mm - 0.6).abs() < f64::EPSILON,
            "NetClassAndLayer should win: expected preferred=0.6, got {}",
            bounds.preferred_mm
        );

        // Power on layer 1 → NetClass wins (preferred=0.3).
        let bounds = policy.width_bounds(Some("Power"), Some(LayerId(1)));
        assert!(
            (bounds.preferred_mm - 0.3).abs() < f64::EPSILON,
            "NetClass should win for Power on layer 1: expected preferred=0.3, got {}",
            bounds.preferred_mm
        );

        // Signal on layer 0 → All wins (preferred=0.2).
        let bounds = policy.width_bounds(Some("Signal"), Some(LayerId(0)));
        assert!(
            (bounds.preferred_mm - 0.2).abs() < f64::EPSILON,
            "All should win for Signal on layer 0: expected preferred=0.2, got {}",
            bounds.preferred_mm
        );
    }

    #[test]
    fn no_matching_scope_falls_back_to_all() {
        let mut ir = empty_ir();
        // Only an All-scope rule exists.
        add_scoped_width_rule(&mut ir, 1, IrRuleScope::All, 0.15, 2.0, 0.25);

        let policy = DrcPolicy::build(&ir).unwrap();

        // Any combination of net class / layer falls through to All.
        let bounds = policy.width_bounds(Some("HighSpeed"), Some(LayerId(0)));
        assert!(
            (bounds.preferred_mm - 0.25).abs() < f64::EPSILON,
            "Should fall back to All: expected preferred=0.25, got {}",
            bounds.preferred_mm
        );
    }
}
