//! PCB differential pair record type.
//!
//! Differential pairs define two nets that should be routed together
//! with controlled impedance and matched lengths for high-speed signals.
//!
//! **DEPRECATED**: Use `v2::pcb::PcbDifferentialPairV2` with `v2::pcb::io` instead.

use crate::types::{Layer, ParameterCollection};

/// PCB differential pair record.
///
/// A differential pair links two nets (positive and negative) that should
/// be routed as a pair with controlled spacing and length matching.
/// Common uses include USB, HDMI, PCIe, and other high-speed interfaces.
///
/// **DEPRECATED**: Use `v2::pcb::PcbDifferentialPairV2` instead.
#[deprecated(note = "Use v2::pcb::PcbDifferentialPairV2")]
#[derive(Debug, Clone, Default)]
pub struct PcbDifferentialPair {
    /// Differential pair name (e.g., "USB.USB_D").
    pub name: String,
    /// Positive net name (e.g., "USB.USB_D_P").
    pub positive_net_name: String,
    /// Negative net name (e.g., "USB.USB_D_N").
    pub negative_net_name: String,
    /// Layer (typically TOP for default).
    pub layer: Layer,
    /// Whether the pair is locked.
    pub locked: bool,
    /// Whether this is a polygon outline.
    pub polygon_outline: bool,
    /// Whether user-routed.
    pub user_routed: bool,
    /// Whether this is a keepout.
    pub keepout: bool,
    /// Union index (for grouping).
    pub union_index: i32,
    /// Whether gather control is enabled.
    pub gather_control: bool,
    /// Unique ID.
    pub unique_id: String,
    /// All parameters for round-tripping.
    pub params: ParameterCollection,
}

impl PcbDifferentialPair {
    /// Create a new differential pair.
    pub fn new(name: &str, positive_net: &str, negative_net: &str) -> Self {
        Self {
            name: name.to_string(),
            positive_net_name: positive_net.to_string(),
            negative_net_name: negative_net.to_string(),
            ..Default::default()
        }
    }

    /// Parse a differential pair from parameters.
    pub fn from_params(params: &ParameterCollection) -> Self {
        Self {
            name: params
                .get("NAME")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            positive_net_name: params
                .get("POSITIVENETNAME")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            negative_net_name: params
                .get("NEGATIVENETNAME")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            layer: params
                .get("LAYER")
                .map(|v| v.as_layer())
                .unwrap_or_default(),
            locked: params
                .get("LOCKED")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
            polygon_outline: params
                .get("POLYGONOUTLINE")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
            user_routed: params
                .get("USERROUTED")
                .map(|v| v.as_bool_or(true))
                .unwrap_or(true),
            keepout: params
                .get("KEEPOUT")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
            union_index: params
                .get("UNIONINDEX")
                .map(|v| v.as_int_or(0))
                .unwrap_or(0),
            gather_control: params
                .get("GATHERCONTROL")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
            unique_id: params
                .get("UNIQUEID")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            params: params.clone(),
        }
    }

    /// Export to parameters.
    pub fn to_params(&self) -> ParameterCollection {
        let mut params = self.params.clone();

        params.add("NAME", &self.name);
        params.add("POSITIVENETNAME", &self.positive_net_name);
        params.add("NEGATIVENETNAME", &self.negative_net_name);
        params.add("LAYER", &self.layer.to_string());
        params.add("LOCKED", if self.locked { "TRUE" } else { "FALSE" });
        params.add(
            "POLYGONOUTLINE",
            if self.polygon_outline {
                "TRUE"
            } else {
                "FALSE"
            },
        );
        params.add(
            "USERROUTED",
            if self.user_routed { "TRUE" } else { "FALSE" },
        );
        params.add("KEEPOUT", if self.keepout { "TRUE" } else { "FALSE" });
        params.add_int("UNIONINDEX", self.union_index);
        params.add(
            "GATHERCONTROL",
            if self.gather_control { "TRUE" } else { "FALSE" },
        );
        if !self.unique_id.is_empty() {
            params.add("UNIQUEID", &self.unique_id);
        }

        params
    }
}
