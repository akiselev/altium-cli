//! PCB net record type.
//!
//! Nets define electrical connectivity and routing properties
//! for connected copper elements on the PCB.

use crate::types::{Color, Coord, Layer, ParameterCollection};

/// PCB net record.
///
/// A net represents an electrical connection between multiple points
/// on the PCB. It stores routing properties like track widths for
/// different layers and visual display settings.
#[derive(Debug, Clone, Default)]
pub struct PcbNet {
    /// Net name.
    pub name: String,
    /// Layer (typically TOP for default).
    pub layer: Layer,
    /// Whether the net is locked.
    pub locked: bool,
    /// Whether the net is visible.
    pub visible: bool,
    /// Whether primitives are locked.
    pub primitive_lock: bool,
    /// Whether this is a user-routed net.
    pub user_routed: bool,
    /// Whether this is a keepout net.
    pub keepout: bool,
    /// Union index (for grouping).
    pub union_index: i32,
    /// Display color for the net.
    pub color: Color,
    /// Whether to use loop removal optimization.
    pub loop_removal: bool,
    /// Whether to override color for drawing.
    pub override_color_for_draw: bool,
    /// Layer-specific minimum routing widths (indexed by layer).
    /// Format: TOPLAYER_MRWIDTH, MIDLAYER1_MRWIDTH, etc.
    pub layer_widths: Vec<(String, Coord)>,
    /// Unique ID.
    pub unique_id: String,
    /// All parameters for round-tripping.
    pub params: ParameterCollection,
}

impl PcbNet {
    /// Parse a net from parameters.
    pub fn from_params(params: &ParameterCollection) -> Self {
        let mut net = Self {
            name: params
                .get("NAME")
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
            visible: params
                .get("VISIBLE")
                .map(|v| v.as_bool_or(true))
                .unwrap_or(true),
            primitive_lock: params
                .get("PRIMITIVELOCK")
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
            color: params
                .get("COLOR")
                .and_then(|v| v.as_color().ok())
                .unwrap_or_default(),
            loop_removal: params
                .get("LOOPREMOVAL")
                .map(|v| v.as_bool_or(true))
                .unwrap_or(true),
            override_color_for_draw: params
                .get("OVERRIDECOLORFORDRAW")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
            layer_widths: Vec::new(),
            unique_id: params
                .get("UNIQUEID")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            params: params.clone(),
        };

        // Parse layer-specific widths
        // TOPLAYER_MRWIDTH, MIDLAYER1_MRWIDTH, ..., MIDLAYER30_MRWIDTH, BOTTOMLAYER_MRWIDTH
        for (key, value) in params.iter() {
            if key.ends_with("_MRWIDTH") {
                if let Ok(width) = value.as_coord() {
                    let layer_name = key.trim_end_matches("_MRWIDTH").to_string();
                    net.layer_widths.push((layer_name, width));
                }
            }
        }

        net
    }

    /// Export to parameters.
    pub fn to_params(&self) -> ParameterCollection {
        let mut params = self.params.clone();

        params.add("NAME", &self.name);
        params.add("LAYER", &self.layer.to_string());
        params.add("LOCKED", if self.locked { "TRUE" } else { "FALSE" });
        params.add("VISIBLE", if self.visible { "TRUE" } else { "FALSE" });
        params.add(
            "PRIMITIVELOCK",
            if self.primitive_lock { "TRUE" } else { "FALSE" },
        );
        params.add(
            "USERROUTED",
            if self.user_routed { "TRUE" } else { "FALSE" },
        );
        params.add("KEEPOUT", if self.keepout { "TRUE" } else { "FALSE" });
        params.add_int("UNIONINDEX", self.union_index);
        params.add_color("COLOR", self.color);
        params.add(
            "LOOPREMOVAL",
            if self.loop_removal { "TRUE" } else { "FALSE" },
        );
        params.add(
            "OVERRIDECOLORFORDRAW",
            if self.override_color_for_draw {
                "TRUE"
            } else {
                "FALSE"
            },
        );

        // Write layer widths
        for (layer_name, width) in &self.layer_widths {
            params.add_coord(&format!("{}_MRWIDTH", layer_name), *width);
        }

        if !self.unique_id.is_empty() {
            params.add("UNIQUEID", &self.unique_id);
        }

        params
    }

    /// Get the minimum routing width for a specific layer.
    pub fn get_layer_width(&self, layer_name: &str) -> Option<Coord> {
        self.layer_widths
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(layer_name))
            .map(|(_, width)| *width)
    }

    /// Get the top layer minimum routing width.
    pub fn top_layer_width(&self) -> Option<Coord> {
        self.get_layer_width("TOPLAYER")
    }

    /// Get the bottom layer minimum routing width.
    pub fn bottom_layer_width(&self) -> Option<Coord> {
        self.get_layer_width("BOTTOMLAYER")
    }
}
