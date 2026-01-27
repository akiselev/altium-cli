//! PCB from-to record type.
//!
//! From-To records define explicit routing constraints between
//! two pads/pins, specifying which pins should be connected.

use crate::types::{Layer, ParameterCollection};

/// PCB from-to record.
///
/// A from-to specifies a routing constraint between two pins,
/// indicating that they should be directly connected. This is
/// used for xSignals and other advanced routing constraints.
#[derive(Debug, Clone, Default)]
pub struct PcbFromTo {
    /// Source pad/pin designator (e.g., "U1-A1").
    pub from: String,
    /// Destination pad/pin designator (e.g., "U2-B1").
    pub to: String,
    /// Net name.
    pub net: String,
    /// Layer (typically TOP for default).
    pub layer: Layer,
    /// Whether the from-to is locked.
    pub locked: bool,
    /// Whether this is a polygon outline.
    pub polygon_outline: bool,
    /// Whether user-routed.
    pub user_routed: bool,
    /// Whether this is a keepout.
    pub keepout: bool,
    /// Union index (for grouping).
    pub union_index: i32,
    /// Unique ID.
    pub unique_id: String,
    /// All parameters for round-tripping.
    pub params: ParameterCollection,
}

impl PcbFromTo {
    /// Create a new from-to constraint.
    pub fn new(from: &str, to: &str, net: &str) -> Self {
        Self {
            from: from.to_string(),
            to: to.to_string(),
            net: net.to_string(),
            ..Default::default()
        }
    }

    /// Parse a from-to from parameters.
    pub fn from_params(params: &ParameterCollection) -> Self {
        Self {
            from: params
                .get("FROM")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            to: params
                .get("TO")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            net: params
                .get("NET")
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

        params.add("FROM", &self.from);
        params.add("TO", &self.to);
        params.add("NET", &self.net);
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
        if !self.unique_id.is_empty() {
            params.add("UNIQUEID", &self.unique_id);
        }

        params
    }
}
