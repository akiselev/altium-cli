//! Routing configuration types.
//!
//! `RoutingConfig` is the sole input for all router tuning parameters. It is
//! designed for serde deserialization from the `routing { ... }` block in
//! `pcbdoc-spec`. All fields have `#[serde(default)]` so partial JSON is valid.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Corner style for trace routing. This is the router's own type and is
/// independent of `altium_format_types::pcb::CornerStyle`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CornerStyle {
    /// 45-degree chamfered corners (default).
    #[default]
    FortyFiveDegree,
    /// Rounded corners.
    RoundedCorner,
}

/// Movement style for grid-based routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MovementStyle {
    /// Four-way cardinal movement (default).
    #[default]
    FourWay,
    /// Eight-way diagonal movement.
    EightWay,
}

/// Via cost model: base penalty, SI penalty, and per-net-class overrides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViaCostConfig {
    /// Base via cost added to A* path cost.
    #[serde(default = "default_via_cost_base")]
    pub base: f64,

    /// Additional penalty for SI-sensitive nets (differential pairs, high-speed).
    #[serde(default)]
    pub si_penalty: f64,

    /// Per-net-class via cost multiplier overrides. BTreeMap for deterministic
    /// serialization order.
    #[serde(default)]
    pub overrides: BTreeMap<String, f64>,
}

fn default_via_cost_base() -> f64 {
    10.0
}

impl Default for ViaCostConfig {
    fn default() -> Self {
        ViaCostConfig {
            base: default_via_cost_base(),
            si_penalty: 0.0,
            overrides: BTreeMap::new(),
        }
    }
}

/// Per-net routing overrides. Applied on top of the default `RoutingConfig` for
/// nets matching a specific net class name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NetRoutingConfig {
    /// Override via cost for this net class.
    #[serde(default)]
    pub via_cost_override: Option<f64>,

    /// Override trace width (mm) for this net class.
    #[serde(default)]
    pub width_override: Option<f64>,

    /// Override clearance (mm) for this net class.
    #[serde(default)]
    pub clearance_override: Option<f64>,

    /// Restrict routing to specific layers for this net class.
    #[serde(default)]
    pub layer_override: Vec<autopcb_routes::LayerId>,
}

fn default_escape_enabled() -> bool {
    true
}

fn default_min_escape_mm() -> f64 {
    0.5
}

fn default_max_escape_mm() -> f64 {
    3.0
}

fn default_min_access_threshold() -> usize {
    3
}

fn default_roi_initial_radius() -> u32 {
    24
}

fn default_roi_retry_multiplier() -> u32 {
    2
}

fn default_stagnation_threshold() -> u32 {
    5
}

fn default_stagnation_max() -> u32 {
    10
}

/// Configuration for pad escape planning (BGA/QFP fanout pre-routing).
///
/// Escape planning runs before the PathFinder loop and pre-routes short
/// escape traces + vias from dense SMD pads to inner layers, creating
/// routable access points for the main detailed router.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EscapeConfig {
    /// Enable pad escape planning. Default true.
    #[serde(default = "default_escape_enabled")]
    pub enabled: bool,

    /// Minimum escape trace length in mm. Default 0.5mm.
    #[serde(default = "default_min_escape_mm")]
    pub min_escape_mm: f64,

    /// Maximum escape trace length in mm. Default 3.0mm.
    #[serde(default = "default_max_escape_mm")]
    pub max_escape_mm: f64,

    /// Minimum number of free access points before escape planning is
    /// skipped for a pad. Pads with >= this many free neighbours are
    /// considered already routable. Default 3.
    #[serde(default = "default_min_access_threshold")]
    pub min_access_threshold: usize,
}

impl Default for EscapeConfig {
    fn default() -> Self {
        EscapeConfig {
            enabled: default_escape_enabled(),
            min_escape_mm: default_min_escape_mm(),
            max_escape_mm: default_max_escape_mm(),
            min_access_threshold: default_min_access_threshold(),
        }
    }
}

fn default_grid_resolution_mm() -> f64 {
    0.25
}

fn default_max_iterations() -> u32 {
    50
}

fn default_pres_fac_multiplier() -> f64 {
    1.5
}

fn default_pres_fac_cap() -> f64 {
    500.0
}

fn default_history_increment() -> f64 {
    1.5
}

fn default_initial_pres_fac() -> f64 {
    0.5
}

fn default_history_decay() -> f64 {
    1.0
}

fn default_hist_weight() -> f64 {
    1.0
}

/// Top-level routing configuration passed to every stage of the router.
///
/// All fields have serde defaults so partial JSON (or an empty `{}`) is valid.
/// The `seed` field is the sole source of non-determinism: same seed + same
/// `PcbIr` always produces an identical `RouteSolution`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoutingConfig {
    /// Grid cell size in mm. Finer grids find tighter channels but use more
    /// memory (quadratic scaling). Default 0.1mm.
    #[serde(default = "default_grid_resolution_mm")]
    pub grid_resolution_mm: f64,

    /// Maximum PathFinder negotiation iterations before giving up. Default 50.
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,

    /// Base via cost added to A* path cost. Default 10.0.
    #[serde(default = "default_via_cost_base")]
    pub via_cost_base: f64,

    /// Present congestion factor growth multiplier per iteration. McMurchie &
    /// Ebeling 1995 §3.2 baseline is 1.15. Default 1.15.
    #[serde(default = "default_pres_fac_multiplier")]
    pub pres_fac_multiplier: f64,

    /// Upper cap for the present congestion factor. Default 100.0.
    #[serde(default = "default_pres_fac_cap")]
    pub pres_fac_cap: f64,

    /// History congestion increment per oversubscribed-node-iteration. Default 1.5.
    #[serde(default = "default_history_increment")]
    pub history_increment: f64,

    /// Starting present-congestion factor. Lower values allow more natural
    /// shortest-path routing in early iterations. Default 0.5.
    #[serde(default = "default_initial_pres_fac")]
    pub initial_pres_fac: f64,

    /// Multiplicative decay applied to the entire history array at the start
    /// of each iteration. Values < 1.0 allow history to "forget" old
    /// congestion, preventing route fossilization. Default 1.0 (no decay).
    #[serde(default = "default_history_decay")]
    pub history_decay: f64,

    /// Weight multiplier for history cost in the A* cost function.
    /// `C(n) = base * dir_penalty * corridor_penalty + hist_weight * (history[n] + edge_hist[n]) + pres_fac * max(0, usage[n] - 1)`
    /// Default 1.0.
    #[serde(default = "default_hist_weight")]
    pub hist_weight: f64,

    /// Corner style applied during post-route optimization. Default FortyFiveDegree.
    #[serde(default)]
    pub corner_style: CornerStyle,

    /// Layers available for routing. Empty means all copper layers are allowed.
    #[serde(default)]
    pub allowed_layers: Vec<autopcb_routes::LayerId>,

    /// Per-net-class routing overrides keyed by net class name. BTreeMap for
    /// deterministic serialization order.
    #[serde(default)]
    pub net_configs: BTreeMap<String, NetRoutingConfig>,

    /// RNG seed for net ordering. ChaCha8Rng is used for platform-stable
    /// determinism. Default 0 gives reproducible behavior.
    #[serde(default)]
    pub seed: u64,

    /// Grid movement style: cardinal (FourWay) or diagonal (EightWay). Default FourWay.
    #[serde(default)]
    pub movement: MovementStyle,

    /// Pad escape planning configuration for BGA/QFP dense components.
    #[serde(default)]
    pub escape: EscapeConfig,

    /// Initial ROI radius in grid cells for A* search. Default 24.
    #[serde(default = "default_roi_initial_radius")]
    pub roi_initial_radius: u32,

    /// ROI retry multiplier: second attempt uses radius × this value. Default 2.
    #[serde(default = "default_roi_retry_multiplier")]
    pub roi_retry_multiplier: u32,

    /// Number of stagnation iterations before escalation (double pres_fac, full rip-up).
    /// Default 5.
    #[serde(default = "default_stagnation_threshold")]
    pub stagnation_threshold: u32,

    /// Number of stagnation iterations before early termination. Default 10.
    #[serde(default = "default_stagnation_max")]
    pub stagnation_max: u32,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        RoutingConfig {
            grid_resolution_mm: default_grid_resolution_mm(),
            max_iterations: default_max_iterations(),
            via_cost_base: default_via_cost_base(),
            pres_fac_multiplier: default_pres_fac_multiplier(),
            pres_fac_cap: default_pres_fac_cap(),
            history_increment: default_history_increment(),
            initial_pres_fac: default_initial_pres_fac(),
            history_decay: default_history_decay(),
            hist_weight: default_hist_weight(),
            corner_style: CornerStyle::default(),
            allowed_layers: Vec::new(),
            net_configs: BTreeMap::new(),
            seed: 0,
            movement: MovementStyle::default(),
            escape: EscapeConfig::default(),
            roi_initial_radius: default_roi_initial_radius(),
            roi_retry_multiplier: default_roi_retry_multiplier(),
            stagnation_threshold: default_stagnation_threshold(),
            stagnation_max: default_stagnation_max(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_expected_values() {
        let cfg = RoutingConfig::default();
        assert!((cfg.grid_resolution_mm - 0.25).abs() < f64::EPSILON);
        assert_eq!(cfg.max_iterations, 50);
        assert!((cfg.via_cost_base - 10.0).abs() < f64::EPSILON);
        assert!((cfg.pres_fac_multiplier - 1.5).abs() < f64::EPSILON);
        assert!((cfg.pres_fac_cap - 500.0).abs() < f64::EPSILON);
        assert!((cfg.history_increment - 1.5).abs() < f64::EPSILON);
        assert!((cfg.initial_pres_fac - 0.5).abs() < f64::EPSILON);
        assert!((cfg.history_decay - 1.0).abs() < f64::EPSILON);
        assert!((cfg.hist_weight - 1.0).abs() < f64::EPSILON);
        assert_eq!(cfg.corner_style, CornerStyle::FortyFiveDegree);
        assert!(cfg.allowed_layers.is_empty());
        assert!(cfg.net_configs.is_empty());
        assert_eq!(cfg.seed, 0);
        assert_eq!(cfg.movement, MovementStyle::FourWay);
    }

    #[test]
    fn config_deserializes_from_full_json() {
        let json = r#"{
            "grid_resolution_mm": 0.05,
            "max_iterations": 100,
            "via_cost_base": 15.0,
            "pres_fac_multiplier": 1.2,
            "pres_fac_cap": 200.0,
            "history_increment": 2.0,
            "corner_style": "rounded_corner",
            "allowed_layers": [0, 1],
            "net_configs": {
                "Power": {
                    "via_cost_override": 5.0,
                    "width_override": 0.5,
                    "layer_override": []
                }
            },
            "seed": 42,
            "movement": "eight_way"
        }"#;

        let cfg: RoutingConfig = serde_json::from_str(json).expect("deserialization failed");
        assert!((cfg.grid_resolution_mm - 0.05).abs() < f64::EPSILON);
        assert_eq!(cfg.max_iterations, 100);
        assert!((cfg.via_cost_base - 15.0).abs() < f64::EPSILON);
        assert!((cfg.pres_fac_multiplier - 1.2).abs() < f64::EPSILON);
        assert!((cfg.pres_fac_cap - 200.0).abs() < f64::EPSILON);
        assert!((cfg.history_increment - 2.0).abs() < f64::EPSILON);
        assert_eq!(cfg.corner_style, CornerStyle::RoundedCorner);
        assert_eq!(cfg.seed, 42);
        assert_eq!(cfg.movement, MovementStyle::EightWay);
        assert!(cfg.net_configs.contains_key("Power"));
        let power = &cfg.net_configs["Power"];
        assert_eq!(power.via_cost_override, Some(5.0));
        assert_eq!(power.width_override, Some(0.5));
    }

    #[test]
    fn config_deserializes_from_minimal_json() {
        let json = r#"{}"#;
        let cfg: RoutingConfig = serde_json::from_str(json).expect("deserialization failed");
        assert!((cfg.grid_resolution_mm - 0.25).abs() < f64::EPSILON);
        assert_eq!(cfg.max_iterations, 50);
        assert_eq!(cfg.corner_style, CornerStyle::FortyFiveDegree);
        assert_eq!(cfg.movement, MovementStyle::FourWay);
        assert_eq!(cfg.seed, 0);
        assert!((cfg.initial_pres_fac - 0.5).abs() < f64::EPSILON);
        assert!((cfg.history_decay - 1.0).abs() < f64::EPSILON);
        assert!((cfg.hist_weight - 1.0).abs() < f64::EPSILON);
    }
}
