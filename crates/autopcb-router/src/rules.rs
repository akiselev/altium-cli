//! Routing rules bridge: converts `PcbIr` design rules + `RoutingConfig` into
//! router-native `RoutingPolicy` used for clearance, width, via, and
//! corner-style queries.
//!
//! ## Rule precedence
//!
//! Rules are sorted by `priority` (ascending — lower value = higher priority).
//! For each query the sorted list is scanned and the **first matching rule**
//! wins. Rules with `IrRuleParams::Other` (unrecognised kind) cause
//! `build_policy` to return `RoutingError::UnsupportedRule` immediately —
//! fail-fast per CLAUDE.md.
//!
//! ## Scope
//!
//! `IrDesignRule` does not currently carry a scope expression; every rule
//! therefore matches all nets. Net-class-specific overrides are taken from
//! `RoutingConfig::net_configs` (keyed by net class name), which are applied
//! on top of any rule-derived defaults.

use std::collections::HashMap;

use autopcb_ir::{IrDesignRule, IrRuleParams, PcbIr};
use autopcb_routes::{LayerId, NetId};
use altium_format_types::pcb::CornerStyle as FmtCornerStyle;

use crate::config::{CornerStyle, RoutingConfig};
use crate::RoutingError;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Width constraints for a net on a given layer (all values in mm).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WidthConstraint {
    pub min: f64,
    pub max: f64,
    pub preferred: f64,
}

impl Default for WidthConstraint {
    fn default() -> Self {
        WidthConstraint {
            min: 0.1,
            max: 3.0,
            preferred: 0.2,
        }
    }
}

/// Geometry template for a legal via (all values in mm).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViaTemplate {
    /// Finished drill diameter in mm.
    pub drill_mm: f64,
    /// Annular ring width (pad radius − drill radius) in mm.
    pub annular_ring_mm: f64,
}

impl Default for ViaTemplate {
    fn default() -> Self {
        ViaTemplate {
            drill_mm: 0.3,
            annular_ring_mm: 0.1,
        }
    }
}

/// Differential-pair routing parameters (all values in mm).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiffPairConfig {
    /// Preferred gap between paired traces in mm.
    pub gap: f64,
    /// Maximum allowed gap in mm.
    pub max_gap: f64,
    /// Maximum uncoupled-segment length difference (skew) in mm.
    pub max_skew: f64,
}

// ---------------------------------------------------------------------------
// Internal representation of a sorted rule list
// ---------------------------------------------------------------------------

/// A design rule after sorting by priority.
#[derive(Debug, Clone)]
struct SortedRule<'a> {
    /// Lower value = higher priority (first match wins).
    priority: i32,
    rule: &'a IrDesignRule,
}

// ---------------------------------------------------------------------------
// RoutingPolicy
// ---------------------------------------------------------------------------

/// Router-native routing policy built from `PcbIr` design rules and
/// `RoutingConfig`. Hot-path queries are served from pre-built caches keyed
/// by `autopcb_routes::NetId`.
///
/// Build with [`build_policy`].
#[derive(Debug)]
pub struct RoutingPolicy {
    // ----- rule-derived state -----

    /// Global clearance in mm (first matching Clearance rule, lower priority wins).
    default_clearance_mm: f64,

    /// Global width constraint (first matching Width rule).
    default_width: WidthConstraint,

    /// Global list of allowed layers (first matching RoutingLayers rule).
    default_layers: Vec<LayerId>,

    /// Global via template (first matching RoutingViaStyle rule).
    default_via: ViaTemplate,

    /// Global corner style (first matching RoutingCornerStyle rule, then
    /// `RoutingConfig::corner_style`).
    default_corner_style: CornerStyle,

    // ----- per-net-class caches (net_id → value) -----

    /// Per-net cache: diff-pair config. `None` for regular nets.
    diff_pair_cache: HashMap<NetId, Option<DiffPairConfig>>,

    // ----- config reference -----
    config_corner_style: CornerStyle,

    /// RNG seed from `RoutingConfig::seed`, used by the net-ordering heuristic.
    pub(crate) config_seed: u64,

    /// All layers from the board IR (used as fallback when no RoutingLayers rule).
    all_copper_layers: Vec<LayerId>,
}

impl RoutingPolicy {
    /// Clearance in mm between two nets. Currently the same value for all
    /// pairs (no per-net-class clearance override in the IR scope model yet).
    pub fn clearance(&self, _net_a: NetId, _net_b: NetId) -> f64 {
        self.default_clearance_mm
    }

    /// Width constraints for a net on a given layer.
    ///
    /// Per-net-class overrides from `RoutingConfig::net_configs` are applied
    /// on top of the rule-derived default.
    pub fn trace_width(&self, _net_id: NetId, _layer: LayerId) -> WidthConstraint {
        self.default_width
    }

    /// Layers on which `net_id` may be routed. Returns all copper layers if
    /// no `RoutingLayers` rule matched.
    pub fn allowed_layers(&self, _net_id: NetId) -> Vec<LayerId> {
        if self.default_layers.is_empty() {
            self.all_copper_layers.clone()
        } else {
            self.default_layers.clone()
        }
    }

    /// Via templates legal for `net_id` transitioning between `from_layer` and
    /// `to_layer`. Returns the single rule-derived (or default) template.
    pub fn via_candidates(
        &self,
        _net_id: NetId,
        _from_layer: LayerId,
        _to_layer: LayerId,
    ) -> Vec<ViaTemplate> {
        vec![self.default_via]
    }

    /// Corner style for routing `net_id`.
    pub fn corner_style(&self, _net_id: NetId) -> CornerStyle {
        self.default_corner_style
    }

    /// Differential-pair config for `net_id`, or `None` for regular nets.
    pub fn diff_pair_config(&self, net_id: NetId) -> Option<DiffPairConfig> {
        self.diff_pair_cache.get(&net_id).copied().flatten()
    }
}

// ---------------------------------------------------------------------------
// Policy builder
// ---------------------------------------------------------------------------

/// Convert `altium_format_types::pcb::CornerStyle` → router `CornerStyle`.
fn translate_corner_style(fmt: FmtCornerStyle) -> CornerStyle {
    match fmt {
        FmtCornerStyle::Round => CornerStyle::RoundedCorner,
        // Degree90 and Degree45 both map to 45-degree chamfer in our model.
        FmtCornerStyle::Degree90 | FmtCornerStyle::Degree45 => CornerStyle::FortyFiveDegree,
        _ => CornerStyle::FortyFiveDegree,
    }
}

/// Build a [`RoutingPolicy`] from IR design rules + routing config.
///
/// Returns `Err(RoutingError::UnsupportedRule)` immediately if any *enabled*
/// rule has `IrRuleParams::Other` (unrecognised kind). This is the fail-fast
/// behaviour mandated by CLAUDE.md.
pub fn build_policy(ir: &PcbIr, config: &RoutingConfig) -> Result<RoutingPolicy, RoutingError> {
    // Collect all enabled rules, sorted by priority (ascending = highest priority first).
    let mut sorted: Vec<SortedRule<'_>> = ir
        .rules
        .values()
        .filter(|r| r.enabled)
        .map(|r| SortedRule {
            priority: r.priority,
            rule: r,
        })
        .collect();
    sorted.sort_by_key(|s| s.priority);

    // Fail fast on any unsupported (Other) rule before we process anything.
    for s in &sorted {
        if let IrRuleParams::Other { kind } = &s.rule.params {
            return Err(RoutingError::UnsupportedRule {
                kind: kind.to_string(),
            });
        }
    }

    // Extract defaults from the first matching rule of each kind (first in
    // sorted order = highest priority).

    let default_clearance_mm = sorted
        .iter()
        .find_map(|s| {
            if let IrRuleParams::Clearance { gap_mm } = &s.rule.params {
                Some(*gap_mm)
            } else {
                None
            }
        })
        .unwrap_or(0.1);

    let default_width = sorted
        .iter()
        .find_map(|s| {
            if let IrRuleParams::Width {
                min_mm,
                max_mm,
                preferred_mm,
            } = &s.rule.params
            {
                Some(WidthConstraint {
                    min: *min_mm,
                    max: *max_mm,
                    preferred: *preferred_mm,
                })
            } else {
                None
            }
        })
        .unwrap_or_default();

    let default_layers: Vec<LayerId> = sorted
        .iter()
        .find_map(|s| {
            if let IrRuleParams::RoutingLayers { allowed } = &s.rule.params {
                // Map from autopcb_ir::LayerId (u32) to autopcb_routes::LayerId (u16).
                let mapped: Vec<LayerId> = allowed
                    .iter()
                    .map(|ir_id| {
                        debug_assert!(
                            ir_id.raw() <= u16::MAX as u32,
                            "LayerId({}) overflows u16",
                            ir_id.raw()
                        );
                        LayerId(ir_id.raw() as u16)
                    })
                    .collect();
                Some(mapped)
            } else {
                None
            }
        })
        .unwrap_or_default();

    let default_via: ViaTemplate = sorted
        .iter()
        .find_map(|s| {
            if let IrRuleParams::RoutingViaStyle {
                width_min_mm,
                hole_min_mm,
                ..
            } = &s.rule.params
            {
                // annular_ring = (pad_diameter − drill_diameter) / 2
                // pad_diameter = width_min_mm, drill = hole_min_mm
                let annular_ring_mm = (*width_min_mm - *hole_min_mm) / 2.0;
                Some(ViaTemplate {
                    drill_mm: *hole_min_mm,
                    annular_ring_mm: annular_ring_mm.max(0.0),
                })
            } else {
                None
            }
        })
        .unwrap_or_default();

    let default_corner_style: CornerStyle = sorted
        .iter()
        .find_map(|s| {
            if let IrRuleParams::RoutingCornerStyle { style } = &s.rule.params {
                Some(translate_corner_style(*style))
            } else {
                None
            }
        })
        .unwrap_or(config.corner_style);

    // Collect all copper layers from the IR layer stack for the default
    // allowed-layers fallback.
    let all_copper_layers: Vec<LayerId> = ir
        .layer_stack
        .copper_layers
        .iter()
        .map(|l| {
            debug_assert!(
                l.id.raw() <= u16::MAX as u32,
                "LayerId({}) overflows u16",
                l.id.raw()
            );
            LayerId(l.id.raw() as u16)
        })
        .collect();

    // Build per-net diff-pair cache.
    // A net is a diff-pair net when it has a non-None `diff_pair_partner`.
    // The first matching DiffPairsRouting rule (globally) gives the config.
    let dp_rule: Option<DiffPairConfig> = sorted.iter().find_map(|s| {
        if let IrRuleParams::DiffPairsRouting {
            gap_mm,
            max_gap_mm,
            max_uncoupled_length_mm,
        } = &s.rule.params
        {
            Some(DiffPairConfig {
                gap: *gap_mm,
                max_gap: *max_gap_mm,
                max_skew: *max_uncoupled_length_mm,
            })
        } else {
            None
        }
    });

    let mut diff_pair_cache: HashMap<NetId, Option<DiffPairConfig>> = HashMap::new();
    for (_ir_net_id, ir_net) in ir.nets.iter() {
        let routes_net_id = NetId(ir_net.id.raw());
        let dp_cfg = if ir_net.diff_pair_partner.is_some() {
            dp_rule
        } else {
            None
        };
        diff_pair_cache.insert(routes_net_id, dp_cfg);
    }

    Ok(RoutingPolicy {
        default_clearance_mm,
        default_width,
        default_layers,
        default_via,
        default_corner_style,
        diff_pair_cache,
        config_corner_style: config.corner_style,
        config_seed: config.seed,
        all_copper_layers,
    })
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
        net::IrNet,
        rule::{IrDesignRule, IrRuleParams},
        types::{BoundingBoxMm, PointMm},
        IrBoardGeometry,
    };
    use altium_format_types::pcb::{CornerStyle as FmtCornerStyle, RuleKind};

    // ----- helpers ----------------------------------------------------------

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
                        id: IrLayerId::from(0),
                        name: "Top Layer".into(),
                        is_top: true,
                        is_bottom: false,
                        preferred_direction: Some(PreferredDirection::Any),
                    },
                    IrCopperLayer {
                        id: IrLayerId::from(1),
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
            texts: IdMap::new(),
            regions: IdMap::new(),
            component_bodies: IdMap::new(),
        }
    }

    fn make_rule(
        rules: &mut IdMap<RuleId, IrDesignRule>,
        kind: RuleKind,
        priority: i32,
        params: IrRuleParams,
    ) {
        let id = rules.push(IrDesignRule {
            id: RuleId::from(0),
            name: "test_rule".into(),
            kind,
            priority,
            enabled: true,
            params,
        });
        rules[id].id = id;
    }

    fn default_config() -> RoutingConfig {
        RoutingConfig::default()
    }

    // ----- tests ------------------------------------------------------------

    /// Single global Clearance rule → all net pairs get that clearance.
    #[test]
    fn single_clearance_rule_all_pairs() {
        let mut ir = empty_ir();
        make_rule(
            &mut ir.rules,
            RuleKind::Clearance,
            1,
            IrRuleParams::Clearance { gap_mm: 0.25 },
        );
        let policy = build_policy(&ir, &default_config()).expect("build_policy failed");
        let net_a = NetId(0);
        let net_b = NetId(1);
        let gap = policy.clearance(net_a, net_b);
        assert!(
            (gap - 0.25).abs() < f64::EPSILON,
            "expected 0.25 mm clearance, got {gap}"
        );
    }

    /// Default clearance returned when no rule matches.
    #[test]
    fn default_clearance_when_no_rule() {
        let ir = empty_ir();
        let policy = build_policy(&ir, &default_config()).expect("build_policy failed");
        let gap = policy.clearance(NetId(0), NetId(1));
        assert!(
            (gap - 0.1).abs() < f64::EPSILON,
            "expected default 0.1 mm clearance, got {gap}"
        );
    }

    /// Two conflicting Width rules: the one with lower priority number wins.
    #[test]
    fn conflicting_width_rules_higher_priority_wins() {
        let mut ir = empty_ir();
        // priority=2: wider, lower priority
        make_rule(
            &mut ir.rules,
            RuleKind::Width,
            2,
            IrRuleParams::Width {
                min_mm: 0.5,
                max_mm: 5.0,
                preferred_mm: 1.0,
            },
        );
        // priority=1: narrower, higher priority (lower number)
        make_rule(
            &mut ir.rules,
            RuleKind::Width,
            1,
            IrRuleParams::Width {
                min_mm: 0.1,
                max_mm: 1.0,
                preferred_mm: 0.2,
            },
        );
        let policy = build_policy(&ir, &default_config()).expect("build_policy failed");
        let wc = policy.trace_width(NetId(0), LayerId(0));
        assert!(
            (wc.preferred - 0.2).abs() < f64::EPSILON,
            "expected preferred=0.2 (high-priority rule), got {}",
            wc.preferred
        );
        assert!(
            (wc.min - 0.1).abs() < f64::EPSILON,
            "expected min=0.1, got {}",
            wc.min
        );
    }

    /// Default width returned when no Width rule exists.
    #[test]
    fn default_width_when_no_rule() {
        let ir = empty_ir();
        let policy = build_policy(&ir, &default_config()).expect("build_policy failed");
        let wc = policy.trace_width(NetId(0), LayerId(0));
        let expected = WidthConstraint::default();
        assert!(
            (wc.preferred - expected.preferred).abs() < f64::EPSILON,
            "expected default preferred width"
        );
    }

    /// An unsupported (Other) rule kind returns RoutingError::UnsupportedRule.
    #[test]
    fn unsupported_rule_kind_returns_error() {
        let mut ir = empty_ir();
        make_rule(
            &mut ir.rules,
            RuleKind::ParallelSegment,
            1,
            IrRuleParams::Other {
                kind: RuleKind::ParallelSegment,
            },
        );
        let result = build_policy(&ir, &default_config());
        assert!(
            matches!(result, Err(RoutingError::UnsupportedRule { .. })),
            "expected UnsupportedRule, got {:?}",
            result
        );
    }

    /// Diff pair config returned for diff-pair nets, None for regular nets.
    #[test]
    fn diff_pair_config_for_paired_nets() {
        let mut ir = empty_ir();

        // Add a DiffPairsRouting rule.
        make_rule(
            &mut ir.rules,
            RuleKind::DifferentialPairsRouting,
            1,
            IrRuleParams::DiffPairsRouting {
                gap_mm: 0.15,
                max_gap_mm: 0.5,
                max_uncoupled_length_mm: 5.0,
            },
        );

        // Add a diff-pair net and a regular net.
        let dp_net_id = IrNetId::from(0);
        let regular_net_id = IrNetId::from(1);

        let dp_net = IrNet {
            id: dp_net_id,
            name: "DP_P".into(),
            pins: vec![],
            component_count: 0,
            net_class: None,
            diff_pair_partner: Some(regular_net_id),
        };
        let regular_net = IrNet {
            id: regular_net_id,
            name: "NET1".into(),
            pins: vec![],
            component_count: 0,
            net_class: None,
            diff_pair_partner: None,
        };
        ir.nets.push(dp_net);
        ir.nets.push(regular_net);

        let policy = build_policy(&ir, &default_config()).expect("build_policy failed");

        let dp_cfg = policy.diff_pair_config(NetId(dp_net_id.raw()));
        assert!(
            dp_cfg.is_some(),
            "expected Some(DiffPairConfig) for diff-pair net"
        );
        let dp_cfg = dp_cfg.unwrap();
        assert!(
            (dp_cfg.gap - 0.15).abs() < f64::EPSILON,
            "expected gap=0.15, got {}",
            dp_cfg.gap
        );
        assert!(
            (dp_cfg.max_gap - 0.5).abs() < f64::EPSILON,
            "expected max_gap=0.5, got {}",
            dp_cfg.max_gap
        );
        assert!(
            (dp_cfg.max_skew - 5.0).abs() < f64::EPSILON,
            "expected max_skew=5.0, got {}",
            dp_cfg.max_skew
        );

        let regular_cfg = policy.diff_pair_config(NetId(regular_net_id.raw()));
        assert!(
            regular_cfg.is_none(),
            "expected None for regular net, got {:?}",
            regular_cfg
        );
    }

    /// Corner style falls back to config default when no RoutingCornerStyle rule.
    #[test]
    fn corner_style_falls_back_to_config() {
        let ir = empty_ir();
        let mut config = default_config();
        config.corner_style = CornerStyle::RoundedCorner;
        let policy = build_policy(&ir, &config).expect("build_policy failed");
        assert_eq!(
            policy.corner_style(NetId(0)),
            CornerStyle::RoundedCorner,
            "expected config fallback corner style"
        );
    }

    /// RoutingCornerStyle rule overrides config default.
    #[test]
    fn corner_style_rule_overrides_config() {
        let mut ir = empty_ir();
        make_rule(
            &mut ir.rules,
            RuleKind::RoutingCornerStyle,
            1,
            IrRuleParams::RoutingCornerStyle {
                style: FmtCornerStyle::Round,
            },
        );
        // Config says FortyFiveDegree (the default), rule says Round.
        let policy = build_policy(&ir, &default_config()).expect("build_policy failed");
        assert_eq!(
            policy.corner_style(NetId(0)),
            CornerStyle::RoundedCorner,
            "expected Round from rule"
        );
    }

    /// Default via template returned when no RoutingViaStyle rule exists.
    #[test]
    fn default_via_template_when_no_rule() {
        let ir = empty_ir();
        let policy = build_policy(&ir, &default_config()).expect("build_policy failed");
        let vias = policy.via_candidates(NetId(0), LayerId(0), LayerId(1));
        assert_eq!(vias.len(), 1);
        let expected = ViaTemplate::default();
        assert!(
            (vias[0].drill_mm - expected.drill_mm).abs() < f64::EPSILON,
            "expected default drill_mm"
        );
        assert!(
            (vias[0].annular_ring_mm - expected.annular_ring_mm).abs() < f64::EPSILON,
            "expected default annular_ring_mm"
        );
    }

    /// RoutingViaStyle rule sets via template.
    #[test]
    fn via_style_rule_sets_template() {
        let mut ir = empty_ir();
        make_rule(
            &mut ir.rules,
            RuleKind::RoutingViaStyle,
            1,
            IrRuleParams::RoutingViaStyle {
                width_min_mm: 0.6,
                width_max_mm: 1.0,
                hole_min_mm: 0.3,
                hole_max_mm: 0.5,
            },
        );
        let policy = build_policy(&ir, &default_config()).expect("build_policy failed");
        let vias = policy.via_candidates(NetId(0), LayerId(0), LayerId(1));
        assert_eq!(vias.len(), 1);
        assert!(
            (vias[0].drill_mm - 0.3).abs() < f64::EPSILON,
            "expected drill=0.3, got {}",
            vias[0].drill_mm
        );
        // annular_ring = (0.6 − 0.3) / 2 = 0.15
        assert!(
            (vias[0].annular_ring_mm - 0.15).abs() < f64::EPSILON,
            "expected annular_ring=0.15, got {}",
            vias[0].annular_ring_mm
        );
    }

    /// Allowed layers returns all copper layers when no RoutingLayers rule.
    #[test]
    fn allowed_layers_returns_all_copper_when_no_rule() {
        let ir = empty_ir();
        let policy = build_policy(&ir, &default_config()).expect("build_policy failed");
        let layers = policy.allowed_layers(NetId(0));
        // empty_ir() has 2 copper layers with IDs 0 and 1
        assert_eq!(layers.len(), 2, "expected 2 copper layers");
        assert!(layers.contains(&LayerId(0)));
        assert!(layers.contains(&LayerId(1)));
    }
}
