//! Via cost model: base penalty, SI penalty, per-net-class overrides.
//!
//! `ViaCostModel` is queried during A* successor expansion to price layer
//! transitions. BTreeMap overrides for deterministic serialization.

use std::collections::BTreeMap;

use crate::config::RoutingConfig;

/// Cost model for via (layer-transition) penalty during A* pathfinding.
///
/// The cost of placing a via for a net is:
///   - If the net class matches an entry in `overrides`: use that value.
///   - Otherwise: `base + si_penalty`.
///
/// `overrides` uses `BTreeMap` for deterministic serialization order (matching
/// the plan decision for `ViaCostModel`).
#[derive(Debug, Clone, PartialEq)]
pub struct ViaCostModel {
    /// Base via penalty added to A* path cost.
    pub base: f64,
    /// Additional penalty for SI-sensitive nets (differential pairs,
    /// high-speed).
    pub si_penalty: f64,
    /// Per-net-class cost overrides. Key is the net class name.
    /// BTreeMap for deterministic iteration order.
    pub overrides: BTreeMap<String, f64>,
}

impl ViaCostModel {
    /// Return the via cost for a net belonging to `net_class`.
    ///
    /// Checks `overrides` by net class name first; falls back to
    /// `base + si_penalty`.
    pub fn cost(&self, net_class: Option<&str>) -> f64 {
        if let Some(class) = net_class {
            if let Some(&override_cost) = self.overrides.get(class) {
                return override_cost;
            }
        }
        self.base + self.si_penalty
    }

    /// Build a `ViaCostModel` from the top-level routing config.
    ///
    /// - `base` comes from `config.via_cost_base`.
    /// - `si_penalty` defaults to 0.0 (no SI rule in config yet).
    /// - `overrides` are populated from `config.net_configs` entries that have
    ///   a `via_cost_override`.
    pub fn from_config(config: &RoutingConfig) -> Self {
        let overrides: BTreeMap<String, f64> = config
            .net_configs
            .iter()
            .filter_map(|(class, nc)| nc.via_cost_override.map(|c| (class.clone(), c)))
            .collect();
        ViaCostModel {
            base: config.via_cost_base,
            si_penalty: 0.0,
            overrides,
        }
    }
}

impl Default for ViaCostModel {
    fn default() -> Self {
        ViaCostModel {
            base: 10.0,
            si_penalty: 0.0,
            overrides: BTreeMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{NetRoutingConfig, RoutingConfig};

    #[test]
    fn default_cost_is_base_plus_si_penalty() {
        let model = ViaCostModel {
            base: 10.0,
            si_penalty: 5.0,
            overrides: BTreeMap::new(),
        };
        assert!(
            (model.cost(None) - 15.0).abs() < f64::EPSILON,
            "expected base + si_penalty = 15.0, got {}",
            model.cost(None)
        );
    }

    #[test]
    fn no_class_uses_base_plus_si() {
        let model = ViaCostModel {
            base: 8.0,
            si_penalty: 2.0,
            overrides: BTreeMap::from([("Power".to_string(), 4.0)]),
        };
        assert!(
            (model.cost(None) - 10.0).abs() < f64::EPSILON,
            "no class should give base + si_penalty = 10.0"
        );
    }

    #[test]
    fn net_class_override_used_when_present() {
        let model = ViaCostModel {
            base: 10.0,
            si_penalty: 5.0,
            overrides: BTreeMap::from([("HighSpeed".to_string(), 25.0)]),
        };
        assert!(
            (model.cost(Some("HighSpeed")) - 25.0).abs() < f64::EPSILON,
            "override should be used for HighSpeed class"
        );
    }

    #[test]
    fn unknown_class_falls_back_to_default() {
        let model = ViaCostModel {
            base: 10.0,
            si_penalty: 3.0,
            overrides: BTreeMap::from([("Power".to_string(), 4.0)]),
        };
        assert!(
            (model.cost(Some("Signal")) - 13.0).abs() < f64::EPSILON,
            "unknown class should fall back to base + si_penalty"
        );
    }

    #[test]
    fn from_config_reads_via_cost_base() {
        let mut config = RoutingConfig::default();
        config.via_cost_base = 20.0;
        let model = ViaCostModel::from_config(&config);
        assert!(
            (model.base - 20.0).abs() < f64::EPSILON,
            "base should match config.via_cost_base"
        );
        assert!(
            model.si_penalty.abs() < f64::EPSILON,
            "si_penalty should default to 0.0"
        );
    }

    #[test]
    fn from_config_populates_overrides_from_net_configs() {
        let mut config = RoutingConfig::default();
        config.net_configs.insert(
            "Power".to_string(),
            NetRoutingConfig {
                via_cost_override: Some(5.0),
                ..Default::default()
            },
        );
        config.net_configs.insert(
            "Signal".to_string(),
            NetRoutingConfig {
                via_cost_override: None,
                ..Default::default()
            },
        );
        let model = ViaCostModel::from_config(&config);
        assert!(
            model.overrides.contains_key("Power"),
            "Power override should be present"
        );
        assert!(
            (model.overrides["Power"] - 5.0).abs() < f64::EPSILON
        );
        assert!(
            !model.overrides.contains_key("Signal"),
            "Signal has no override so should not appear"
        );
    }
}
